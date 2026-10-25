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


def from_fix(decls: list[Decl], trial: list[Decl], improved: bool, message: str, span=None) -> Suggestion:
    return Suggestion(message, classify(decls, trial, improved), render(trial), span)
