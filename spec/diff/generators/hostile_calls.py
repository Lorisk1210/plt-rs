"""The argument list of a generated call bound to its callable's
signature, so that the ``hostile`` generator can change one parameter's
value and leave the others as they were."""

from __future__ import annotations

import ast
import inspect

#: The parameter kinds a call can give positionally.
POSITIONAL = (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)


class Arguments:
    """A call's arguments (``text``, as a call writes them), each with the
    parameter it binds."""

    def __init__(self, text: str, parameters: list[inspect.Parameter]) -> None:
        self.parameters = {p.name: p for p in parameters}
        self.variadic = any(p.kind is inspect.Parameter.VAR_POSITIONAL for p in parameters)
        source = f"(0)({text})"
        call = ast.parse(source, mode="eval").body
        #: (keyword or None, source) of each argument, in order
        self.items: list[tuple[str | None, str]] = []
        #: parameter -> the index of the item giving it
        self.bound: dict[str, int] = {}
        self.starred = any(isinstance(a, ast.Starred) for a in call.args) or any(k.arg is None for k in call.keywords)
        positional = [p.name for p in parameters if p.kind in POSITIONAL]
        for number, argument in enumerate(call.args):
            self.items.append((None, ast.get_source_segment(source, argument)))
            if not self.starred and number < len(positional):
                self.bound[positional[number]] = len(self.items) - 1
        for keyword in call.keywords:
            self.items.append((keyword.arg, ast.get_source_segment(source, keyword.value)))
            if keyword.arg is not None:
                self.bound[keyword.arg] = len(self.items) - 1

    def positional_count(self) -> int:
        """How many arguments the call gives positionally."""
        return sum(1 for keyword, _ in self.items if keyword is None)

    def value_of(self, name: str) -> str | None:
        """The source the call gives ``name``, if it gives it."""
        at = self.bound.get(name)
        return None if at is None else self.items[at][1]

    def text(self, items: list[tuple[str | None, str]] | None = None) -> str:
        """The arguments as a call writes them."""
        return ", ".join(value if keyword is None else f"{keyword}={value}"
                         for keyword, value in (self.items if items is None else items))

    def with_value(self, name: str, value: str) -> str | None:
        """The arguments with ``name`` given ``value`` (None if the call
        cannot give it: a positional-only parameter after a gap)."""
        value = _one(value)
        items = list(self.items)
        at = self.bound.get(name)
        if at is not None:
            items[at] = (items[at][0], value)
            return self.text(items)
        parameter = self.parameters.get(name)
        if parameter is None or self.starred:
            return None
        if parameter.kind is inspect.Parameter.POSITIONAL_ONLY:
            positional = [p for p in self.parameters.values() if p.kind in POSITIONAL]
            if positional.index(parameter) != self.positional_count():
                return None
            keywords = [item for item in items if item[0] is not None]
            return self.text([item for item in items if item[0] is None] + [(None, value)] + keywords)
        return self.text(items + [(name, value)])

    def with_keyword(self, name: str, value: str) -> str:
        """The arguments with the keyword ``name=value`` added."""
        return self.text(self.items + [(name, _one(value))])


def _one(value: str) -> str:
    """``value`` as one argument: a sample written as several (a setter's
    ``1, 100, 2``) given as the tuple of them."""
    try:
        call = ast.parse(f"(0)({value})", mode="eval").body
    except SyntaxError:
        return value
    return f"({value})" if len(call.args) + len(call.keywords) > 1 else value
