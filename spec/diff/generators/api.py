"""Every public callable of matplotlib, called by a program of the subject
it belongs to, with each of its keyword parameters.

The callables are those the coverage counts (``api_index``: functions,
classes, methods, properties). Each goes to one subject:

* a module's functions and classes to the module's programs, which import
  it as ``m`` and call them (a class: its constructor);
* a class' members to the first subject (``SUBJECTS``, then the artist
  subjects of ``artists``) whose object finds the member along its
  class' MRO, not overridden: the object of a subclass carries the
  members of its bases.

Per callable, the program calls it once per argument variant: the
variants of ``values.ARGS`` (its own, written out), else the values of
its required parameters (``values.VALUES``, looked up by
``Owner.name.param``, ``name.param`` and ``param``); then each
representative and edge value of each other keyword parameter in turn.
Values naming what a subject lacks (``fig``, say) are left out. A setter
is followed by its getter, a property is read (and set, when settable,
then read again), a dunder method is used as Python uses it (``len``,
``[]``, ``in``, ...). Members that undo the subject (``remove``,
``clear``) come last, each on a subject made anew. A subject may have
variants (``subjects.VARIANTS``, ``variants_toolkits.VARIANTS``): another
object of its class, made with other arguments, on which its callables
run again.

A subject's callables are split into programs of ``CHUNK``; each program
ends drawing every pyplot figure and saving it in each vector format, so
that the writers run and the images are compared.
"""

from __future__ import annotations

import ast
import builtins
import inspect
import sys
import warnings
from typing import Any, Iterator

import api_index

from . import artists, common, values, variants_toolkits
from .subjects import MODULE_SETUP, PRELUDE, SUBJECTS, VARIANTS

#: Callables per program.
CHUNK = 25
#: Members that undo their subject: each runs last, on a subject made anew.
UNDOING = {"remove", "clear", "cla", "clf", "close", "delaxes", "disconnect", "disconnect_events", "pop",
           "remove_callback", "reset", "__exit__", "__del__", "finalize", "cleanup"}
#: The formats every program saves its figures in at its end.
FORMATS = ("pdf", "svg", "ps", "eps")
#: How Python calls a dunder method: ``{r}`` is the receiver, ``{a}`` the arguments.
DUNDERS = {
    "__call__": "{r}({a})", "__getitem__": "{r}[{a}]", "__len__": "len({r})", "__iter__": "list({r})",
    "__contains__": "({a}) in {r}", "__bool__": "bool({r})", "__array__": "np.asarray({r})",
    "__add__": "{r} + {a}", "__radd__": "{a} + {r}", "__sub__": "{r} - {a}", "__rsub__": "{a} - {r}",
    "__mul__": "{r} * {a}", "__rmul__": "{a} * {r}", "__truediv__": "{r} / {a}", "__neg__": "-{r}",
    "__lt__": "{r} < {a}", "__le__": "{r} <= {a}", "__gt__": "{r} > {a}", "__ge__": "{r} >= {a}",
    "__enter__": "with {r}:\n    pass", "__setitem__": "{r}[{a}]", "__delitem__": "del {r}[{a}]",
    "__getattr__": "getattr({r}, {a})", "__setattr__": "setattr({r}, {a})", "__delattr__": "delattr({r}, {a})",
    "__reversed__": "list(reversed({r}))", "__or__": "{r} | {a}", "__and__": "{r} & {a}",
    "__matmul__": "{r} @ {a}", "__abs__": "abs({r})", "__index__": "[0, 1, 2][{r}]",
    "__int__": "int({r})", "__float__": "float({r})", "__hash__": "hash({r})", "__copy__": "copy.copy({r})",
    "__deepcopy__": "copy.deepcopy({r})", "__get__": "{a}", "__set_name__": "{a}",
}


def _free_names(expression: str) -> set[str]:
    """The names ``expression`` (an argument list, or a subscript's
    index) reads that it does not bind itself."""
    tree = _arguments(expression) or ast.parse(f"(0)[{expression}]", mode="eval")
    read, bound = set(), set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Name):
            (read if isinstance(node.ctx, ast.Load) else bound).add(node.id)
        elif isinstance(node, ast.arg):
            bound.add(node.arg)
    return read - bound


class Scope:
    """The names a subject's program has: what its statements bind under
    matplotlib, and the builtins."""

    def __init__(self, namespace: dict[str, Any]) -> None:
        self.names = set(namespace) | set(dir(builtins))

    def available(self, expressions: list[str]) -> list[str]:
        """The ``expressions`` reading only names the program has."""
        return [e for e in expressions if _free_names(e) <= self.names]


def _arguments(arguments: str) -> ast.Expression | None:
    """``arguments`` parsed as a call's (None if they are a subscript's)."""
    try:
        return ast.parse(f"(0)({arguments})", mode="eval")
    except SyntaxError:
        return None


def _positional_count(arguments: str) -> int | None:
    """How many positional arguments ``arguments`` gives (None with a
    ``*args``, which may give any, or for a subscript)."""
    tree = _arguments(arguments)
    if tree is None or any(isinstance(a, ast.Starred) for a in tree.body.args):
        return None
    return len(tree.body.args)


def _keywords(arguments: str) -> set[str]:
    tree = _arguments(arguments)
    return set() if tree is None else {k.arg for k in tree.body.keywords if k.arg}


def _keyword(name: str, value: str) -> str:
    """``name=value``, a value that is an argument list (a setter's sample,
    ``1, 100, 2``) given as the tuple of its arguments."""
    tree = _arguments(value)
    several = tree is not None and len(tree.body.args) + len(tree.body.keywords) > 1
    return f"{name}=({value})" if several else f"{name}={value}"


def _join(*parts: str) -> str:
    return ", ".join(p for p in parts if p)


def _parameters(callable_: api_index.Callable) -> list[inspect.Parameter]:
    target = callable_.obj.__func__ if callable_.kind in ("classmethod", "staticmethod") else callable_.obj
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            parameters = list(inspect.signature(target).parameters.values())
    except (TypeError, ValueError):
        return []
    if callable_.kind in ("method", "classmethod") and parameters and parameters[0].kind in (
            inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD):
        parameters = parameters[1:]
    return parameters


def _values(callable_: api_index.Callable, parameter: str, scope: Scope) -> list[str]:
    """The values a program gives ``parameter`` of ``callable_``, the
    representative first."""
    keys = [f"{callable_.name}.{parameter}", parameter]
    if callable_.owner:
        keys.insert(0, f"{callable_.owner}.{callable_.name}.{parameter}")
    for key in keys:
        if key in values.VALUES:
            return scope.available(values.VALUES[key])
    if callable_.name.startswith("set_") and parameter not in artists.VALUES:
        return scope.available(artists.VALUES.get(callable_.name[4:], []))
    return scope.available(artists.VALUES.get(parameter, []))


def argument_variants(callable_: api_index.Callable, scope: Scope) -> list[str]:
    """The argument lists a program calls ``callable_`` with (none if its
    required parameters have no values)."""
    parameters = _parameters(callable_)
    explicit = values.args_for(callable_)
    if explicit is not None:
        variants = scope.available(explicit)
        if not variants:
            return []
    else:
        positional, keyword, alternatives = [], [], []
        for parameter in parameters:
            if parameter.default is not inspect.Parameter.empty or parameter.kind in (
                    inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD):
                continue
            found = _values(callable_, parameter.name, scope)
            if not found:
                return []
            keyword_only = parameter.kind is inspect.Parameter.KEYWORD_ONLY
            (keyword if keyword_only else positional).append(
                _keyword(parameter.name, found[0]) if keyword_only else found[0])
            alternatives.append((keyword_only, len(keyword if keyword_only else positional) - 1,
                                 parameter.name, found[1:]))
        variants = [_join(*positional, *keyword)]
        for keyword_only, at, name, others in alternatives:
            for other in others:
                changed_positional, changed_keyword = list(positional), list(keyword)
                if keyword_only:
                    changed_keyword[at] = _keyword(name, other)
                else:
                    changed_positional[at] = other
                variants.append(_join(*changed_positional, *changed_keyword))
    base = variants[0]
    given = _positional_count(base)
    mentioned = set().union(*map(_keywords, variants))
    for position, parameter in enumerate(parameters):
        if parameter.default is inspect.Parameter.empty or parameter.name in mentioned:
            continue
        if parameter.kind not in api_index.KEYWORD_KINDS:
            continue
        if given is None or (parameter.kind is not inspect.Parameter.KEYWORD_ONLY and position < given):
            continue
        for value in _values(callable_, parameter.name, scope):
            variants.append(_join(base, _keyword(parameter.name, value)))
    return variants


def _unbound(callable_: api_index.Callable, owner: str, scope: Scope) -> list[str]:
    """The statements calling ``callable_``, a member ``obj``'s class
    overrides, through ``owner``, the class defining it (as a subclass'
    ``super()`` would)."""
    name = callable_.name
    if callable_.kind == "property":
        out = [f"{owner}.{name}.fget(obj)"]
        if callable_.obj.fset is not None:
            out += [f"{owner}.{name}.fset(obj, {v})" for v in _values(callable_, name, scope)]
        return out
    first = "" if callable_.kind in ("classmethod", "staticmethod", "class") else "obj"
    return [f"{owner}.{name}({_join(first, a)})" for a in argument_variants(callable_, scope)]


def statements(callable_: api_index.Callable, receiver: str, cls: type, scope: Scope) -> list[str]:
    """The statements calling ``callable_`` on ``receiver`` (of class ``cls``)."""
    name = callable_.name
    if callable_.kind == "property":
        out = [f"{receiver}.{name}"]
        if callable_.obj.fset is not None:
            for value in _values(callable_, name, scope):
                out += [f"{receiver}.{name} = {value}", f"{receiver}.{name}"]
        return out
    variants = argument_variants(callable_, scope)
    if name in DUNDERS:
        if not variants and "{a}" in DUNDERS[name]:
            return []
        return [DUNDERS[name].format(r=receiver, a=a) for a in variants or [""]]
    getter = f"get_{name[4:]}" if name.startswith("set_") else None
    has_getter = getter is not None and callable(getattr(cls, getter, None))
    out = []
    for arguments in variants:
        out.append(f"{receiver}.{name}({arguments})")
        if has_getter:
            out.append(f"{receiver}.{getter}()")
    return out


#: The last statements of every program: every pyplot figure drawn, then
#: saved in each format.
ENDING = ["for number in plt.get_fignums():\n    plt.figure(number).canvas.draw()"] + [
    f"for number in plt.get_fignums():\n    plt.figure(number).savefig(io.BytesIO(), format={f!r})" for f in FORMATS]


class Subject:
    """One subject's program: its statements, what they bind, and the
    callables it calls (with the class defining each, for the members its
    object's class overrides)."""

    def __init__(self, name: str, setup: list[str], receiver: str, namespace: dict[str, Any]) -> None:
        self.name = name
        self.setup = setup
        self.receiver = receiver
        obj = namespace.get("obj")
        self.cls = obj if isinstance(obj, type) else type(obj)
        self.scope = Scope(namespace)
        self.callables: list[tuple[api_index.Callable, str | None]] = []

    def _made(self, callable_: api_index.Callable, owner: str | None) -> list[str]:
        if owner is None:
            return statements(callable_, self.receiver, self.cls, self.scope)
        return _unbound(callable_, owner, self.scope)

    def programs(self) -> Iterator[tuple[str, list[str]]]:
        """The subject's programs, ``CHUNK`` callables each."""
        keeping = [c for c in self.callables if c[0].name not in UNDOING]
        undoing = [c for c in self.callables if c[0].name in UNDOING]
        chunks = [keeping[i:i + CHUNK] for i in range(0, len(keeping), CHUNK)] or [[]]
        chunks[-1] += undoing
        for number, chunk in enumerate(chunks):
            imports, body = [], []
            for callable_, owner in chunk:
                made = self._made(callable_, owner)
                if made and owner is not None and not callable_.owner.startswith("_"):
                    imports.append(f"import {callable_.module}")
                body += (self.setup if callable_.name in UNDOING and made else []) + made
            if body:
                yield f"{self.name}_{number}", PRELUDE + sorted(set(imports)) + self.setup + body + ENDING


def _owner_expression(callable_: api_index.Callable) -> str:
    """How a program names the class defining ``callable_``: by its module,
    or, for a private base, by its name along ``obj``'s MRO."""
    if callable_.owner.startswith("_"):
        return f"next(c for c in type(obj).__mro__ if c.__name__ == {callable_.owner!r})"
    return f"{callable_.module}.{callable_.owner}"


def _namespace(name: str, setup: list[str]) -> dict[str, Any] | None:
    """What the subject's statements bind under matplotlib; None, with the
    reason on stderr, for a subject matplotlib cannot make here (one
    needing TeX on a machine without it, say)."""
    try:
        return common.namespace(PRELUDE + setup)
    except Exception as err:  # noqa: BLE001 - the subject is left out, and says why
        print(f"api: subject {name} left out: {type(err).__name__}: {err}", file=sys.stderr)
        return None


def _subjects(index: api_index.Index) -> list[Subject]:
    """The subjects, each with the callables it calls."""
    subjects, taken = [], set()
    modules: dict[str, list[api_index.Callable]] = {}
    for callable_ in index.callables:
        if not callable_.owner:
            modules.setdefault(callable_.module, []).append(callable_)
    for module, callables in modules.items():
        setup = [f"import {module} as m", *MODULE_SETUP.get(module, [])]
        namespace = _namespace(module, setup)
        if namespace is not None:
            subject = Subject("module_" + module.replace(".", "_"), setup, "m", namespace)
            subject.callables = [(c, None) for c in callables]
            taken.update(c.key for c in callables)
            subjects.append(subject)
    objects = []
    for name, setup in SUBJECTS + artists.SUBJECTS:
        namespace = _namespace(name, setup)
        if namespace is not None:
            objects.append(Subject(name, setup, "obj", namespace))
    # Members as the object's class finds them, then the members its
    # class overrides, through the class defining them.
    for overridden in (False, True):
        for subject in objects:
            for base in subject.cls.__mro__:
                for member in vars(base):
                    callable_ = index.by_member.get((id(base), member))
                    if callable_ is None or callable_.key in taken:
                        continue
                    found = index.member(subject.cls, member) is callable_
                    if found is overridden:
                        continue
                    taken.add(callable_.key)
                    subject.callables.append((callable_, _owner_expression(callable_) if overridden else None))
    return subjects + objects + _variants(objects)


def _variants(objects: list[Subject]) -> list[Subject]:
    """The subjects of ``VARIANTS``: each calls its base subject's
    callables on an object made with other arguments."""
    bases = {}
    for subject in objects:
        if subject.callables:
            bases.setdefault(subject.name, subject)
    variants = []
    for name, base, setup in VARIANTS + variants_toolkits.VARIANTS:
        namespace = _namespace(name, setup)
        if namespace is not None and base in bases:
            variant = Subject(name, setup, "obj", namespace)
            variant.callables = list(bases[base].callables)
            variants.append(variant)
    return variants


def programs() -> Iterator[tuple[str, list[str]]]:
    """The programs of every module and every subject."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        index = api_index.Index(api_index.enumerate_api(sys.executable))
    for subject in _subjects(index):
        yield from subject.programs()
