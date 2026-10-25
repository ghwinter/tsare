"""Suggestions with an applicability, borrowed from rustc.

rustc marks every suggestion with an `Applicability` so a tool knows whether it may apply it
without asking.  Ours were prose, which is why a model could not act on them (note 48).
The four levels are the same:

    MachineApplicable  verified to improve the document and introduces no placeholder
    MaybeIncorrect     verified to improve it, but it adds a claim the author did not write
    HasPlaceholders    the replacement contains `?`, so a person must fill it in
    Unspecified        nothing verified it

A suggestion replaces a span.  Our repairs rewrite whole declarations, so the span is the
declaration's own span and the replacement is its rendered text.
"""

from __future__ import annotations

import enum

from fix import render
from syntax import Decl


class Applicability(enum.IntEnum):
    MachineApplicable = 0
    MaybeIncorrect = 1
    HasPlaceholders = 2
    Unspecified = 3

    def __str__(self) -> str:
        return self.name


class Suggestion:
    def __init__(self, message: str, applicability: Applicability, replacement: str, span=None):
        self.message = message
        self.applicability = applicability
        self.replacement = replacement
        self.span = span

    def __repr__(self) -> str:
        return f"Suggestion({self.message!r}, {self.applicability})"


def classify(decls: list[Decl], trial: list[Decl], improved: bool) -> Applicability:
    """The replacement's applicability, judged from the document it produces."""
    if not improved:
        return Applicability.Unspecified
    before = render(decls).count("?")
    after = render(trial).count("?")
    if after > before:
        return Applicability.HasPlaceholders
    added = {d.name for d in trial} - {d.name for d in decls}
    if added:
        return Applicability.MaybeIncorrect
    return Applicability.MachineApplicable


def changed(decls: list[Decl], trial: list[Decl]) -> tuple[list[Decl], list[str]]:
    """(declarations to replace, names added).  A declaration counts as changed when its
    rendered text differs; this keeps the suggestion to the smallest span it can name."""
    before = {d.name: render([d]) for d in decls}
    replacement = []
    added = []
    for decls_item in trial:
        if decls_item.name not in before:
            added.append(decls_item.name)
        elif before[decls_item.name] != render([decls_item]):
            replacement.append(decls_item)
    return replacement, added


def from_fix(decls: list[Decl], trial: list[Decl], improved: bool, message: str, span=None) -> Suggestion:
    replacement, added = changed(decls, trial)
    import collections

    before = collections.Counter(render([d]) for d in decls)
    after = collections.Counter(render([d]) for d in trial)
    removed = [d for d in decls if after[render([d])] < before[render([d])]]
    if not replacement and len(removed) == 1 and not added:
        return Suggestion(message, classify(decls, trial, improved), "", removed[0].span)
    if len(replacement) == 1:
        span = replacement[0].span
        text = render(replacement)
    else:
        text = render(trial)
    return Suggestion(message, classify(decls, trial, improved), text, span)
