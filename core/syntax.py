"""Front end: spans, lexer, AST, recursive-descent parser for the `.mf` surface language.

The surface language is deliberately small: every declaration is

    <kind> <id> { <field>: <value>, ... }

Values are strings, numbers, identifiers, lists, records, and calls.  The parser is
error-recovering: a malformed declaration is reported, then tokens are skipped until
the next line-initial identifier, so one typo does not hide the rest of the file.
"""

from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(frozen=True)
class Span:
    line: int
    col: int
    end_line: int
    end_col: int


@dataclass
class Token:
    kind: str  # IDENT | STRING | NUMBER | punct | EOF
    text: str
    span: Span


PUNCT = set("{}[]():,?")
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
NUM_RE = re.compile(r"\d+(?:\.\d+)?")


class SyntaxError_(Exception):
    def __init__(self, message: str, span: Span):
        super().__init__(message)
        self.message = message
        self.span = span


# --------------------------------------------------------------------------- lexer


def lex(src: str) -> list[Token]:
    toks: list[Token] = []
    i, line, col, n = 0, 1, 1, len(src)
    while i < n:
        ch = src[i]
        if ch == "\n":
            i += 1
            line += 1
            col = 1
            continue
        if ch in " \t\r":
            i += 1
            col += 1
            continue
        if ch == "#":
            while i < n and src[i] != "\n":
                i += 1
            continue
        if ch in PUNCT:
            toks.append(Token(ch, ch, Span(line, col, line, col + 1)))
            i += 1
            col += 1
            continue
        if ch == '"':
            start_col = col
            j = i + 1
            buf: list[str] = []
            while j < n and src[j] != '"':
                if src[j] == "\n":
                    raise SyntaxError_(
                        "unterminated string literal", Span(line, start_col, line, start_col + 1)
                    )
                if src[j] == "\\":
                    if j + 1 >= n:
                        raise SyntaxError_(
                            "unterminated escape sequence",
                            Span(line, start_col, line, start_col + 1),
                        )
                    esc = src[j + 1]
                    buf.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\"}.get(esc, esc))
                    j += 2
                else:
                    buf.append(src[j])
                    j += 1
            if j >= n:
                raise SyntaxError_(
                    "unterminated string literal", Span(line, start_col, line, start_col + 1)
                )
            width = j - i + 1
            toks.append(
                Token("STRING", "".join(buf), Span(line, start_col, line, start_col + width))
            )
            i = j + 1
            col += width
            continue
        m = NUM_RE.match(src, i)
        if m and (ch.isdigit() or (ch == "-" and i + 1 < n and src[i + 1].isdigit())):
            digits = m.group(0)
            j = i + len(digits)
            # people write `8ms`, so a unit suffix is part of the number, not a new token
            suffix = re.match(r"[A-Za-z%]+", src[j:])
            unit = suffix.group(0) if suffix else ""
            text = digits + unit
            toks.append(Token("NUMBER", text, Span(line, col, line, col + len(text))))
            i += len(text)
            col += len(text)
            continue
        m = IDENT_RE.match(src, i)
        if m:
            text = m.group(0)
            toks.append(Token("IDENT", text, Span(line, col, line, col + len(text))))
            i += len(text)
            col += len(text)
            continue
        raise SyntaxError_(f"unexpected character {ch!r}", Span(line, col, line, col + 1))
    toks.append(Token("EOF", "", Span(line, col, line, col)))
    return toks


# ----------------------------------------------------------------------------- ast


@dataclass
class Val:
    span: Span


@dataclass
class VStr(Val):
    text: str


@dataclass
class VNum(Val):
    value: float
    raw: str
    unit: str = ""


@dataclass
class VId(Val):
    name: str


@dataclass
class VList(Val):
    items: list[Val]


@dataclass
class VRec(Val):
    fields: dict[str, Val]
    order: list[str]


@dataclass
class VCall(Val):
    name: str
    args: list[Val]


@dataclass
class VHole(Val):
    """`?` -- an explicitly unknown value. It satisfies any type and becomes a question."""


@dataclass
class Decl:
    name: str
    kind: str
    fields: dict[str, Val]
    order: list[str]
    span: Span
    kind_span: Span


def describe(val: Val) -> str:
    if isinstance(val, VStr):
        return "string"
    if isinstance(val, VNum):
        return "number"
    if isinstance(val, VId):
        return "identifier"
    if isinstance(val, VList):
        return "list"
    if isinstance(val, VRec):
        return "record"
    if isinstance(val, VCall):
        return "call"
    return "value"


# -------------------------------------------------------------------------- parser


class Parser:
    def __init__(self, toks: list[Token]):
        self.toks = toks
        self.i = 0

    def peek(self, k: int = 0) -> Token:
        return self.toks[min(self.i + k, len(self.toks) - 1)]

    def at(self, kind: str, text: str | None = None) -> bool:
        tok = self.peek()
        return tok.kind == kind and (text is None or tok.text == text)

    def take(self, kind: str, text: str | None = None) -> Token:
        tok = self.peek()
        if tok.kind != kind or (text is not None and tok.text != text):
            want = text if text is not None else kind
            raise SyntaxError_(f"expected {want!r}, found {tok.text!r}", tok.span)
        self.i += 1
        return tok

    # -- declarations ---------------------------------------------------------

    def parse_program(self) -> tuple[list[Decl], list[tuple[str, Span]]]:
        decls: list[Decl] = []
        errors: list[tuple[str, Span]] = []
        while not self.at("EOF"):
            try:
                decls.append(self.parse_decl())
            except SyntaxError_ as exc:
                errors.append((exc.message, exc.span))
                self.sync()
        return decls, errors

    def sync(self) -> None:
        """Skip to the next line-initial identifier (or EOF)."""
        while not self.at("EOF"):
            tok = self.peek()
            if tok.kind == "IDENT" and tok.span.col == 1:
                return
            self.i += 1

    def parse_decl(self) -> Decl:
        kind_tok = self.take("IDENT")
        name_tok = self.take("IDENT")
        span = kind_tok.span
        fields, order, end = self.parse_record()
        return Decl(
            name=name_tok.text,
            kind=kind_tok.text,
            fields=fields,
            order=order,
            span=Span(span.line, span.col, end.end_line, end.end_col),
            kind_span=kind_tok.span,
        )

    def parse_record(self) -> tuple[dict[str, Val], list[str], Span]:
        open_tok = self.take("{")
        fields: dict[str, Val] = {}
        order: list[str] = []
        while not self.at("}"):
            key = self.take("IDENT")
            self.take(":")
            val = self.parse_value()
            if key.text in fields:
                raise SyntaxError_(f"duplicate field {key.text!r}", key.span)
            fields[key.text] = val
            order.append(key.text)
            if self.at(","):
                self.take(",")
            elif not self.at("}"):
                raise SyntaxError_(
                    f"expected ',' or '}}' after field {key.text!r}", self.peek().span
                )
        close = self.take("}")
        return fields, order, close.span

    def parse_value(self) -> Val:
        tok = self.peek()
        if tok.kind == "?":
            self.take("?")
            return VHole(tok.span)
        if tok.kind == "STRING":
            self.take("STRING")
            return VStr(tok.span, tok.text)
        if tok.kind == "NUMBER":
            self.take("NUMBER")
            parts = re.match(r"(\d+(?:\.\d+)?)([A-Za-z%]*)$", tok.text)
            assert parts, tok.text
            return VNum(tok.span, float(parts.group(1)), tok.text, parts.group(2))
        if tok.kind == "[":
            open_tok = self.take("[")
            items: list[Val] = []
            while not self.at("]"):
                items.append(self.parse_value())
                if self.at(","):
                    self.take(",")
                elif not self.at("]"):
                    raise SyntaxError_("expected ',' or ']' in list", self.peek().span)
            close = self.take("]")
            return VList(Span(open_tok.span.line, open_tok.span.col, close.span.end_line, close.span.end_col), items)
        if tok.kind == "{":
            fields, order, end = self.parse_record()
            return VRec(Span(tok.span.line, tok.span.col, end.end_line, end.end_col), fields, order)
        if tok.kind == "IDENT":
            self.take("IDENT")
            if self.at("("):
                self.take("(")
                args: list[Val] = []
                while not self.at(")"):
                    args.append(self.parse_value())
                    if self.at(","):
                        self.take(",")
                    elif not self.at(")"):
                        raise SyntaxError_("expected ',' or ')' in call", self.peek().span)
                close = self.take(")")
                return VCall(
                    Span(tok.span.line, tok.span.col, close.span.end_line, close.span.end_col),
                    tok.text,
                    args,
                )
            return VId(tok.span, tok.text)
        raise SyntaxError_(f"expected a value, found {tok.text!r}", tok.span)


def parse(src: str) -> tuple[list[Decl], list[tuple[str, Span]]]:
    try:
        toks = lex(src)
    except SyntaxError_ as exc:
        return [], [(exc.message, exc.span)]
    return Parser(toks).parse_program()


def load(path) -> tuple[list[Decl], list[tuple[str, Span]], str]:
    text = path.read_text(encoding="utf-8")
    decls, errors = parse(text)
    return decls, errors, text
