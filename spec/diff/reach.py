"""Runs one program under matplotlib and records which public callables it
reaches, for the coverage of the differential tester (``coverage.py``).

    python reach.py API_JSONL MODE PROGRAM OUT_DIR

The program runs as the differential tester runs it (``trace.py``, in
``program`` or ``steps`` mode), with two more changes to its code:

* every call in it becomes ``__reach_call__(callable, name, *args,
  **kwargs)`` (``name`` the attribute called, which tells an alias such as
  ``plot3D`` from the method it is), which notes the public callable the
  call reaches (``api_index``) and
  the parameters the call binds (by keyword or position, and the keys of
  a ``**kwargs``), then makes the call; a call its signature refuses
  reaches nothing;
  and the kinds of the values it gives them (``kinds.of``); a call the
  signature refuses, or that fails on an unexpected keyword, is noted as
  refused, with why;
* every attribute read becomes ``__reach_attribute__(value, name)``,
  which notes the enumeration and exception classes a program refers to
  without calling them, and the properties it reads (those of
  matplotlib's C++ extensions, and the deprecated ones, whose getters
  ``sys.monitoring`` sees called by a wrapper, too).

What Python calls implicitly (properties, dunder methods) is found with
``sys.monitoring``: the start of one of their codes counts when the frame
calling it is the program's. Only what the program calls itself counts,
not what matplotlib calls on its behalf. The choices matplotlib's
``_api.check_in_list`` and ``_api.getitem_checked`` check a value against
count for the parameter of the program's call the value was given to,
however deep down the check is. The calls go to
``OUT_DIR/reach.json``; the trace's own record to ``OUT_DIR/trace/``.
"""

from __future__ import annotations

import ast
import builtins
import enum
import functools
import importlib.util
import inspect
import json
import pathlib
import sys
import types
from typing import Any

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import api_index  # noqa: E402
import kinds  # noqa: E402

CALL, ATTRIBUTE = "__reach_call__", "__reach_attribute__"
#: The longest string value noted as a choice given.
MAX_CHOICE = 60
#: How a refusal of a call reads, by the start of the ``TypeError`` binding it.
REFUSALS = {"got an unexpected keyword": "unknown keyword", "missing a required": "missing argument",
            "too many positional": "too many arguments", "multiple values": "argument given twice"}
#: Calls never wrapped: their meaning depends on the frame they run in.
FRAME_CALLS = {"super", "locals", "globals", "vars", "dir", "eval", "exec", "__import__"}


class Wrapping(ast.NodeTransformer):
    """Turns the calls and attribute reads of a program into calls of the
    recording functions."""

    def visit_Call(self, node: ast.Call) -> Any:
        """``f(*args, **kwargs)`` as ``__reach_call__(f, name, *args,
        **kwargs)``, ``name`` the attribute ``f`` is read as (or None)."""
        func = node.func
        if isinstance(func, ast.Name) and (func.id in FRAME_CALLS or func.id.startswith("__difftest_")):
            self.generic_visit(node)
            return node
        name = None
        if isinstance(func, ast.Attribute):
            # The callee's object is read as it is (a method stays a method);
            # its receiver is still visited.
            func.value = self.visit(func.value)
            name = func.attr
        else:
            func = self.visit(func)
        args = [self.visit(a) for a in node.args]
        keywords = [self.visit(k) for k in node.keywords]
        call = ast.Call(ast.Name(CALL, ast.Load()), [func, ast.Constant(name), *args], keywords)
        return ast.copy_location(call, node)

    def visit_Attribute(self, node: ast.Attribute) -> Any:
        """``value.name`` read as ``__reach_attribute__(value, 'name')``."""
        self.generic_visit(node)
        mangled = node.attr.startswith("__") and not node.attr.endswith("__")
        if not isinstance(node.ctx, ast.Load) or mangled:
            return node
        read = ast.Call(ast.Name(ATTRIBUTE, ast.Load()), [node.value, ast.Constant(node.attr)], [])
        return ast.copy_location(read, node)


class Reach:
    """The public callables one run reaches, and with which parameters."""

    def __init__(self, index: api_index.Index, program: pathlib.Path) -> None:
        self.index = index
        self.program = str(program)
        self.directory = str(program.parent) + "/"
        #: key -> parameters bound
        self.calls: dict[str, set[str]] = {}
        #: key -> keys given to its **kwargs
        self.extra: dict[str, set[str]] = {}
        #: key -> parameter (``**key`` for a **kwargs' key) -> kinds of the values given
        self.kinds: dict[str, dict[str, set[str]]] = {}
        #: key -> parameter -> the strings given
        self.strings: dict[str, dict[str, set[str]]] = {}
        #: key -> parameter -> the choices matplotlib checked the values given against
        self.choices: dict[str, dict[str, set[str]]] = {}
        #: key -> why calls of it were refused
        self.refused: dict[str, set[str]] = {}
        #: The program's calls running, innermost last: key and bound arguments.
        self._running: list[tuple[str, dict[str, Any]]] = []
        #: (id of a function, whether bound) -> the function and its signature
        self._signatures: dict[tuple[int, bool], tuple[Any, inspect.Signature | None]] = {}
        self._program_files: dict[str, bool] = {}

    def is_program(self, filename: str) -> bool:
        """Whether code of ``filename`` is the program's (or this
        module's recording functions, which run for it)."""
        known = self._program_files.get(filename)
        if known is None:
            known = filename in (self.program, __file__) or filename.startswith(self.directory)
            self._program_files[filename] = known
        return known

    def note(self, callable_: api_index.Callable, params: set[str] = frozenset(), extra: set[str] = frozenset()) -> None:
        """Note that ``callable_`` was reached with ``params`` bound."""
        self.calls.setdefault(callable_.key, set()).update(params)
        if extra:
            self.extra.setdefault(callable_.key, set()).update(extra)

    def _signature(self, target: Any) -> inspect.Signature | None:
        """``target``'s signature, cached by the function a bound method
        binds (a bound method is made anew for each call, and its id is
        soon another's), which the cache keeps alive."""
        inner = getattr(target, "__func__", target)
        key = (id(inner), inspect.ismethod(target))
        if key not in self._signatures:
            try:
                signature = inspect.signature(target)
            except (TypeError, ValueError):
                signature = None
            self._signatures[key] = (inner, signature)
        return self._signatures[key][1]

    def _bound(self, target: Any, args: tuple, kwargs: dict[str, Any]) -> dict[str, Any] | str:
        """The arguments a call binds, by parameter (a ``**kwargs``' keys as
        ``**key``), or why its signature refuses it, which reaches nothing."""
        signature = self._signature(target)
        if signature is None:
            return {name: value for name, value in kwargs.items()}
        try:
            bound = signature.bind(*args, **kwargs).arguments
        except TypeError as err:
            return next((why for start, why in REFUSALS.items() if start in str(err)), "refused")
        arguments = {}
        for name, value in bound.items():
            kind = signature.parameters[name].kind
            if kind is inspect.Parameter.VAR_KEYWORD:
                arguments.update((f"**{key}", given) for key, given in value.items())
            elif kind is not inspect.Parameter.VAR_POSITIONAL:
                arguments[name] = value
        return arguments

    def _note_arguments(self, callable_: api_index.Callable, arguments: dict[str, Any]) -> None:
        """Note the parameters a call of ``callable_`` binds, and the kinds
        of their values."""
        params = {name for name in arguments if not name.startswith("**")} & set(callable_.params)
        extra = {name[2:] for name in arguments if name.startswith("**")}
        self.note(callable_, params, extra)
        kinds_given = self.kinds.setdefault(callable_.key, {})
        for name, value in arguments.items():
            if name in params or name.startswith("**"):
                kinds_given.setdefault(name, set()).update(kinds.of(value))
                if isinstance(value, str) and len(value) <= MAX_CHOICE:
                    self.strings.setdefault(callable_.key, {}).setdefault(name, set()).add(value)

    def checked(self, values: Any, kwargs: dict[str, Any]) -> None:
        """Note the choices of a check of matplotlib's for the parameter of
        a running call of the program's that was given the value checked."""
        for name, value in kwargs.items():
            for key, arguments in reversed(self._running):
                parameter = next((p for p in (name, f"**{name}") if p in arguments), None)
                given = arguments.get(parameter)
                if parameter is not None and (given is value or (
                        isinstance(value, str) and isinstance(given, str) and given.lower() == value.lower())):
                    self.choices.setdefault(key, {}).setdefault(parameter, set()).update(kinds.choices_of(values))
                    break

    def _called(self, func: Any, called_as: str | None) -> tuple[api_index.Callable | None, Any, tuple]:
        """The callable a call of ``func`` (read as the attribute
        ``called_as``) reaches, the object whose signature binds the call,
        and the arguments put before the call's."""
        index = self.index
        found = index.by_object.get(id(func))
        if found:
            return _named(found, called_as), func, ()
        if isinstance(func, type):
            # A class of the program's counts as its first public base.
            module = sys.modules.get(func.__module__)
            if func.__module__ != "__main__" and not self.is_program(getattr(module, "__file__", None) or ""):
                return None, None, ()
            base = next((b for b in func.__mro__ if id(b) in index.classes), None)
            return (index.classes[id(base)] if base is not None else None), func, ()
        receiver = getattr(func, "__self__", None)
        name = getattr(func, "__name__", None)
        if receiver is not None and name is not None and not isinstance(receiver, types.ModuleType):
            inner = getattr(func, "__func__", None)
            owner = receiver if isinstance(receiver, type) else type(receiver)
            candidates = index.by_object.get(id(inner), []) if inner is not None else []
            mro = owner.__mro__
            ranked = sorted((c for c in candidates if c.kind != "class" or name == "__init__"),
                            key=lambda c: next((i for i, b in enumerate(mro) if b.__qualname__ == c.owner), len(mro)))
            if name == "__init__" and candidates:
                return candidates[0], inner, (receiver,)
            callable_ = _named(ranked, called_as) if ranked else index.member(owner, name)
            if callable_ is None:
                return None, None, ()
            if inner is not None and inspect.isfunction(inner) and callable_.kind != "classmethod":
                return callable_, inner, (receiver,)
            return callable_, func, ()
        if not inspect.isroutine(func) and callable(func) and not isinstance(func, types.ModuleType):
            callable_ = index.member(type(func), "__call__")
            return callable_, type(func).__call__, (func,)
        return None, None, ()

    def call(self, func: Any, called_as: str | None, /, *args: Any, **kwargs: Any) -> Any:
        """Note what calling ``func`` (read as the attribute ``called_as``)
        reaches, then call it. A call its signature refuses counts as
        refused when it fails, and with its keywords when it does not (a
        signature narrower than the function: ``Artist.set``'s lists the
        properties, not their aliases)."""
        callable_, arguments, refusal = None, {}, None
        try:
            callable_, target, before = self._called(func, called_as)
            bound = None if callable_ is None else self._bound(target, before + args, kwargs)
            if isinstance(bound, str):
                refusal = bound
            elif bound is not None:
                arguments = bound
                self._note_all(callable_, target, arguments)
        except Exception:  # noqa: BLE001 - an object Python cannot introspect reaches nothing
            pass
        if callable_ is None:
            return func(*args, **kwargs)
        self._running.append((callable_.key, arguments))
        try:
            result = func(*args, **kwargs)
        except (TypeError, AttributeError) as err:
            if refusal is not None:
                self.refused.setdefault(callable_.key, set()).add(refusal)
            elif "unexpected keyword argument" in str(err):
                self.refused.setdefault(callable_.key, set()).add("unknown keyword")
            raise
        finally:
            self._running.pop()
        if refusal is not None:
            self._note_all(callable_, target, {name if name in callable_.params else f"**{name}": value
                                               for name, value in kwargs.items()})
        return result

    def _note_all(self, callable_: api_index.Callable, target: Any, arguments: dict[str, Any]) -> None:
        """Note the arguments of a call of ``callable_``, and of the same
        function under the same name elsewhere (a module's function a class
        also holds as a static method, ``colors.to_rgb``)."""
        self._note_arguments(callable_, arguments)
        for same in self.index.by_object.get(id(target), ()):
            if same.name == callable_.name and same is not callable_:
                self._note_arguments(same, arguments)

    def attribute(self, value: Any, name: str) -> Any:
        """Note the enumeration and exception classes a read names, and the
        property it reads; read it."""
        if not isinstance(value, type):
            # A property whose getter raises is reached too.
            found = self.index.member(type(value), name)
            if found is not None and found.kind == "property":
                self.note(found)
        result = getattr(value, name)
        for candidate in (value, result):
            if isinstance(candidate, type) and issubclass(candidate, (enum.Enum, BaseException)):
                found = self.index.classes.get(id(candidate))
                if found is not None:
                    self.note(found)
        return result

    def started(self, code: types.CodeType, offset: int) -> Any:
        """``sys.monitoring``'s PY_START: a property accessor or dunder
        method the program's frame calls counts."""
        candidates = self.index.by_code.get(code)
        if candidates is None:
            return sys.monitoring.DISABLE
        frame = sys._getframe(1)
        caller = frame.f_back
        if caller is None or not self.is_program(caller.f_code.co_filename):
            return None
        receiver = frame.f_locals.get(code.co_varnames[0]) if code.co_argcount else None
        mro = receiver.__mro__ if isinstance(receiver, type) else type(receiver).__mro__
        names = [c.__qualname__ for c in mro]
        chosen = min(candidates, key=lambda c: names.index(c.owner) if c.owner in names else len(names))
        self.note(chosen)
        if all(c.key in self.calls for c in candidates):
            return sys.monitoring.DISABLE
        return None

    def as_json(self) -> dict[str, Any]:
        """What was reached."""
        def nested(table: dict[str, dict[str, set[str]]]) -> dict[str, dict[str, list[str]]]:
            return {key: {name: sorted(found) for name, found in sorted(by_name.items())}
                    for key, by_name in sorted(table.items())}

        return {"calls": {key: sorted(params) for key, params in sorted(self.calls.items())},
                "extra": {key: sorted(keys) for key, keys in sorted(self.extra.items())},
                "kinds": nested(self.kinds), "strings": nested(self.strings), "choices": nested(self.choices),
                "refused": {key: sorted(why) for key, why in sorted(self.refused.items())}}


def _named(candidates: list[api_index.Callable], called_as: str | None) -> api_index.Callable:
    """Of the callables one object is (a method and its aliases, the most
    fitting first), the one named as the program called it (the most
    fitting owner's if several are)."""
    first = candidates[0]
    named = [c for c in candidates if c.name == called_as]
    return next((c for c in named if c.owner == first.owner), named[0] if named else first)


def _watch_checks(reach: Reach) -> None:
    """Have matplotlib's choice checks tell ``reach`` what they check."""
    from matplotlib import _api

    for name in kinds.CHOICE_CHECKS:
        check = getattr(_api, name)

        def watched(values: Any, /, *args: Any, _check: Any = check, **kwargs: Any) -> Any:
            try:
                reach.checked(values, {k: v for k, v in kwargs.items() if not k.startswith("_")})
            except Exception:  # noqa: BLE001 - what a check sees is noted when it can be
                pass
            return _check(values, *args, **kwargs)

        setattr(_api, name, functools.wraps(check)(watched))


def _load_trace() -> types.ModuleType:
    spec = importlib.util.spec_from_file_location("difftest_trace", HERE / "trace.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> int:
    """Run the program through ``trace.py`` and write what it reached."""
    api, mode, program, out = sys.argv[1:5]
    program_path = pathlib.Path(program).resolve()
    out_dir = pathlib.Path(out).resolve()
    index = api_index.load(pathlib.Path(api))
    reach = Reach(index, program_path)
    trace = _load_trace()

    instrument = trace.instrument

    def instrumented(source: str, filename: str, impl: str, mode: str) -> ast.Module:
        return ast.fix_missing_locations(Wrapping().visit(instrument(source, filename, impl, mode)))

    def source_to_code(loader: Any, data: Any, path: Any, *, _optimize: int = -1) -> Any:
        tree = trace.ImportChange(trace.IMPORT_RENAMES[loader.impl]).visit(ast.parse(data, path))
        return compile(ast.fix_missing_locations(Wrapping().visit(tree)), path, "exec", dont_inherit=True)

    _watch_checks(reach)
    trace.instrument = instrumented
    trace._ChangedImportsLoader.source_to_code = source_to_code
    setattr(builtins, CALL, reach.call)
    setattr(builtins, ATTRIBUTE, reach.attribute)
    monitoring = sys.monitoring
    tool = monitoring.COVERAGE_ID
    monitoring.use_tool_id(tool, "pltrs-reach")
    monitoring.register_callback(tool, monitoring.events.PY_START, reach.started)
    monitoring.set_events(tool, monitoring.events.PY_START)
    sys.argv = [str(HERE / "trace.py"), "--impl", "matplotlib", "--mode", mode, str(program_path),
                str(out_dir / "trace")]
    try:
        trace.main()
    finally:
        monitoring.set_events(tool, 0)
        monitoring.free_tool_id(tool)
    (out_dir / "reach.json").write_text(json.dumps(reach.as_json()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
