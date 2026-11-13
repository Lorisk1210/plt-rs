"""Reading a library's module for its uses of matplotlib's private names.

Imported by ``private_census.py``. `Reader.read` walks one module's
syntax tree twice: the first walk only binds the module's names (so that
a function sees what the module assigns after it), the second also
records every use of a private name of matplotlib's (`Use`): an
attribute ``x._name`` loaded, called, stored or deleted (and
``getattr``/``setattr``/``hasattr``/``delattr`` with the name spelled
out), a private name imported from matplotlib's modules, a name of a
private module of matplotlib's, and the private members a subclass of
matplotlib's classes defines over its base's.

The receiver ``x`` is followed as far as reading can (`private_types`):
a subclass' ``self`` and ``super()``, names bound by imports, by
assignments, ``for`` and ``with``, by annotations, through the factories
and attributes matplotlib annotates (``plt.subplots()``,
``ax.xaxis``, ``fig.canvas``, ``ax.get_legend()``). A receiver it cannot
follow is *unknown*; a use on it counts only where the module imports
matplotlib or a module of its library that does, and the name is no
private name of the library's own classes, and the use names every
class of matplotlib's defining it.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass
from typing import Callable

import overrides_census as oc
from private_names import Names, is_private
from private_types import Kind, Typer

FOREIGN = Kind("foreign")
#: The builtins reading or writing an attribute named by a string.
ATTRIBUTE_BUILTINS = {"getattr": "read", "hasattr": "read", "setattr": "written", "delattr": "deleted"}


@dataclass(frozen=True)
class Use:
    """One use of a private name: whose (``owner``, a class or module
    qualname; None when the receiver is unknown, ``candidates`` then
    naming the classes defining it), how (``read``, ``called``,
    ``written``, ``deleted``, ``overridden``), by which library, where."""

    name: str
    owner: str | None
    candidates: tuple[str, ...]
    use: str
    library: str
    where: str


def imports_matplotlib(module: oc.Module) -> bool:
    """Whether ``module`` imports matplotlib (or its toolkits)."""
    return any(target.split(".")[0] in ("matplotlib", "mpl_toolkits") for target in module.bindings.values())


def reaches_matplotlib(census: oc.Census, module: oc.Module) -> bool:
    """Whether ``module`` imports matplotlib, or a module of its library
    that does (a library's module often reaches matplotlib's objects
    through the library's own helpers)."""
    if imports_matplotlib(module):
        return True
    for target in module.bindings.values():
        parts = target.split(".")
        for cut in range(len(parts), 0, -1):
            imported = census.modules.get(".".join(parts[:cut]))
            if imported is not None:
                if imported.library == module.library and imports_matplotlib(imported):
                    return True
                break
    return False


class Scope:
    """The names bound in a module, function or comprehension."""

    def __init__(self, parent: Scope | None = None) -> None:
        self.parent = parent
        self.names: dict[str, Kind | None] = {}

    def lookup(self, name: str) -> tuple[bool, Kind | None]:
        """Whether ``name`` is bound here or above, and to what."""
        scope: Scope | None = self
        while scope is not None:
            if name in scope.names:
                return True, scope.names[name]
            scope = scope.parent
        return False, None


class Reader(ast.NodeVisitor):
    """The walker of one module (see the module's docstring)."""

    def __init__(self, census: oc.Census, names: Names, typer: Typer, own_names: set[str],
                 foreign_names: set[str], record: Callable[[Use], None]) -> None:
        self.census = census
        self.names = names
        self.typer = typer
        self.own_names = own_names
        self.foreign_names = foreign_names
        self.record = record
        self.module: oc.Module | None = None
        self.scope = Scope()
        self.recording = False
        self.imports_matplotlib = False
        self.called: set[int] = set()
        self.augmented: set[int] = set()
        self.self_names: dict[str, Kind | None] = {}

    # Walking -------------------------------------------------------------

    def read(self, module: oc.Module) -> None:
        """Record the uses of ``module`` (one of the census's)."""
        self.module = module
        self.imports_matplotlib = reaches_matplotlib(self.census, module)
        self.scope = Scope()
        for recording in (False, True):
            self.recording = recording
            self.called.clear()
            self.augmented.clear()
            self.visit(module.tree)

    def _emit(self, node: ast.AST, name: str, owner: str | None, use: str,
              candidates: tuple[str, ...] = ()) -> None:
        if self.recording and self.module is not None:
            self.record(Use(name, owner, candidates, use, self.module.library,
                            f"{self.module.name}:{getattr(node, 'lineno', 0)}"))

    def visit_FunctionDef(self, node: ast.FunctionDef | ast.AsyncFunctionDef | ast.Lambda) -> None:
        for default in [*node.args.defaults, *(d for d in node.args.kw_defaults if d is not None)]:
            self.visit(default)
        if not isinstance(node, ast.Lambda):
            for decorator in node.decorator_list:
                self.visit(decorator)
        outer, self.scope = self.scope, Scope(self.scope)
        arguments = [*node.args.posonlyargs, *node.args.args, *node.args.kwonlyargs]
        for index, argument in enumerate(arguments):
            kind = self.typer.annotation(argument.annotation, self.module) if argument.annotation else None
            if index == 0 and argument.arg in self.self_names:
                kind = self.self_names[argument.arg]
            self.scope.names[argument.arg] = kind
        for extra in (node.args.vararg, node.args.kwarg):
            if extra is not None:
                self.scope.names[extra.arg] = None
        saved, self.self_names = self.self_names, {}
        body = node.body if isinstance(node.body, list) else [node.body]
        for statement in body:
            self.visit(statement)
        self.self_names = saved
        self.scope = outer

    visit_AsyncFunctionDef = visit_FunctionDef
    visit_Lambda = visit_FunctionDef

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        for expression in [*node.bases, *node.decorator_list]:
            self.visit(expression)
        ancestors = self.census.matplotlib_ancestors(self.module, node) if self.module else []
        if ancestors:
            self._overrides(node, ancestors)
        instance = Kind.instance(*ancestors) if ancestors else None
        saved = self.self_names
        outer, self.scope = self.scope, Scope(self.scope)
        for statement in node.body:
            self.self_names = {}
            if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)) and statement.args.args:
                first = statement.args.args[0].arg
                decorators = {oc.dotted_name(d) for d in statement.decorator_list}
                if "staticmethod" not in decorators:
                    is_class = "classmethod" in decorators or statement.name in ("__new__", "__init_subclass__")
                    self.self_names = {first: (Kind("class", tuple(ancestors)) if is_class and ancestors
                                               else instance)}
            self.visit(statement)
        self.scope = outer
        self.self_names = saved
        self.scope.names[node.name] = Kind("class", tuple(ancestors)) if ancestors else FOREIGN

    def _overrides(self, node: ast.ClassDef, ancestors: list[type]) -> None:
        for member, _ in oc._members(node):
            if not is_private(member):
                continue
            owners = {self.names.defining(cls, member) for cls in ancestors} - {None}
            for owner in sorted(owners):
                self._emit(node, member, owner, "overridden")

    # Binding -------------------------------------------------------------

    def _bind(self, target: ast.expr, kind: Kind | None) -> None:
        if isinstance(target, ast.Name):
            self.scope.names[target.id] = kind
        elif isinstance(target, (ast.Tuple, ast.List)):
            starred = any(isinstance(e, ast.Starred) for e in target.elts)
            for index, element in enumerate(target.elts):
                if isinstance(element, ast.Starred):
                    inner = kind.element() if kind else None
                    self._bind(element.value, Kind("sequence", inner=inner) if inner else None)
                elif kind is not None and kind.what == "tuple" and not starred and index < len(kind.items):
                    self._bind(element, kind.items[index])
                else:
                    self._bind(element, kind.element() if kind else None)

    def visit_Assign(self, node: ast.Assign) -> None:
        self.visit(node.value)
        kind = self.kind(node.value)
        for target in node.targets:
            self.visit(target)
            self._bind(target, kind)

    def visit_AnnAssign(self, node: ast.AnnAssign) -> None:
        if node.value is not None:
            self.visit(node.value)
        self.visit(node.target)
        kind = self.typer.annotation(node.annotation, self.module)
        self._bind(node.target, kind or (self.kind(node.value) if node.value is not None else None))

    def visit_AugAssign(self, node: ast.AugAssign) -> None:
        self.augmented.add(id(node.target))
        self.visit(node.value)
        self.visit(node.target)

    def visit_NamedExpr(self, node: ast.NamedExpr) -> None:
        self.visit(node.value)
        self._bind(node.target, self.kind(node.value))

    def visit_For(self, node: ast.For | ast.AsyncFor) -> None:
        self.visit(node.iter)
        iterated = self.kind(node.iter)
        self.visit(node.target)
        self._bind(node.target, iterated.element() if iterated else None)
        for statement in [*node.body, *node.orelse]:
            self.visit(statement)

    visit_AsyncFor = visit_For

    def visit_With(self, node: ast.With | ast.AsyncWith) -> None:
        for item in node.items:
            self.visit(item.context_expr)
            if item.optional_vars is not None:
                self.visit(item.optional_vars)
                self._bind(item.optional_vars, self.kind(item.context_expr))
        for statement in node.body:
            self.visit(statement)

    visit_AsyncWith = visit_With

    def _comprehension(self, node: ast.ListComp | ast.SetComp | ast.GeneratorExp | ast.DictComp) -> None:
        outer, self.scope = self.scope, Scope(self.scope)
        for generator in node.generators:
            self.visit(generator.iter)
            iterated = self.kind(generator.iter)
            self._bind(generator.target, iterated.element() if iterated else None)
            self.visit(generator.target)
            for condition in generator.ifs:
                self.visit(condition)
        for part in ("elt", "key", "value"):
            if hasattr(node, part):
                self.visit(getattr(node, part))
        self.scope = outer

    visit_ListComp = visit_SetComp = visit_GeneratorExp = visit_DictComp = _comprehension

    # Uses ----------------------------------------------------------------

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if self.module is None:
            return
        source = self.module._absolute(node.module, node.level)
        if source.split(".")[0] not in ("matplotlib", "mpl_toolkits"):
            return
        for alias in node.names:
            self._module_member(node, source, alias.name, "read")

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            parts = alias.name.split(".")
            if parts[0] in ("matplotlib", "mpl_toolkits") and len(parts) > 1:
                self._module_member(node, ".".join(parts[:-1]), parts[-1], "read")

    def _module_member(self, node: ast.AST, module: str, name: str, use: str) -> bool:
        """Record ``module.name`` if it is a private name of matplotlib's."""
        names = self.names.module_names.get(module)
        if names is None or name not in names:
            return False
        self._emit(node, name, module, use)
        return True

    def visit_Name(self, node: ast.Name) -> None:
        if not isinstance(node.ctx, ast.Load) or self.module is None:
            return
        bound, _ = self.scope.lookup(node.id)
        if bound:
            return
        dotted = self.module.resolve_local(node.id)
        if dotted and dotted.split(".")[0] in ("matplotlib", "mpl_toolkits") and "." in dotted:
            module, _, name = dotted.rpartition(".")
            self._module_member(node, module, name, "called" if id(node) in self.called else "read")

    def visit_Call(self, node: ast.Call) -> None:
        self.called.add(id(node.func))
        function = node.func
        if (isinstance(function, ast.Name) and function.id in ATTRIBUTE_BUILTINS and len(node.args) >= 2
                and isinstance(node.args[1], ast.Constant) and isinstance(node.args[1].value, str)
                and is_private(node.args[1].value)):
            self._attribute(node, node.args[0], node.args[1].value, ATTRIBUTE_BUILTINS[function.id])
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        if isinstance(node.ctx, ast.Store):
            uses = ["read", "written"] if id(node) in self.augmented else ["written"]
        elif isinstance(node.ctx, ast.Del):
            uses = ["deleted"]
        else:
            uses = ["called" if id(node) in self.called else "read"]
        receiver = self.kind(node.value)
        if receiver is not None and receiver.what == "module" and any(
                is_private(part) for part in receiver.module.split(".")):
            for use in uses:
                self._module_member(node, receiver.module, node.attr, use)
        elif is_private(node.attr):
            for use in uses:
                self._attribute(node, node.value, node.attr, use, receiver)
        self.generic_visit(node)

    def _attribute(self, node: ast.AST, value: ast.expr, name: str, use: str,
                   receiver: Kind | None = None) -> None:
        receiver = receiver if receiver is not None else self.kind(value)
        if receiver is not None and receiver.what == "module":
            self._module_member(node, receiver.module, name, use)
            return
        if receiver is not None and receiver.what in ("instance", "class"):
            owners = sorted({self.names.defining(cls, name) for cls in receiver.classes} - {None})
            for owner in owners:
                self._emit(node, name, owner, use)
            if owners:
                return
            # A class of matplotlib's whose subclass defines it (an Axes
            # made with a projection): the receiver's class is not known.
            definers = self.names.definers.get(name, set())
            if not any(issubclass(self.names.classes[d], receiver.classes) for d in definers):
                return
        elif receiver is not None:
            return
        self._unknown(node, value, name, use)

    def _unknown(self, node: ast.AST, value: ast.expr, name: str, use: str) -> None:
        definers = self.names.definers.get(name)
        if not definers or not self.imports_matplotlib or name in self.own_names:
            return
        if isinstance(value, ast.Name) and value.id in self.self_names and name in self.foreign_names:
            return
        self._emit(node, name, None, use, tuple(sorted(definers)))

    # Kinds ---------------------------------------------------------------

    def kind(self, node: ast.expr) -> Kind | None:
        """What ``node`` holds, if reading can tell."""
        if isinstance(node, ast.Name):
            bound, kind = self.scope.lookup(node.id)
            if bound:
                return kind
            if node.id in self.self_names:
                return self.self_names[node.id]
            dotted = self.module.resolve_local(node.id) if self.module else None
            if dotted is None:
                return None
            if dotted.split(".")[0] in ("matplotlib", "mpl_toolkits"):
                return self.typer.of_dotted(dotted)
            return self._library_class(dotted)
        if isinstance(node, ast.Attribute):
            owner = self.kind(node.value)
            return self._member(owner, node.attr)
        if isinstance(node, ast.Call):
            return self._called(node)
        if isinstance(node, ast.Subscript):
            owner = self.kind(node.value)
            if owner is None:
                return None
            if isinstance(node.slice, ast.Slice) and owner.what == "sequence":
                return owner
            if owner.what == "instance":
                return self.typer.returned("__getitem__", owner)
            return owner.element()
        if isinstance(node, ast.Tuple):
            return Kind("tuple", items=tuple(self.kind(e) for e in node.elts))
        if isinstance(node, (ast.List, ast.ListComp)):
            elements = [self.kind(e) for e in node.elts] if isinstance(node, ast.List) else [self.kind(node.elt)]
            return Kind("sequence", inner=elements[0]) if elements and elements[0] and len(set(elements)) == 1 \
                else FOREIGN
        if isinstance(node, ast.IfExp):
            body, orelse = self.kind(node.body), self.kind(node.orelse)
            return body if body == orelse or orelse is None else (orelse if body is None else None)
        if isinstance(node, ast.BoolOp):
            kinds = {self.kind(value) for value in node.values} - {None}
            return kinds.pop() if len(kinds) == 1 else None
        if isinstance(node, ast.NamedExpr):
            return self.kind(node.value)
        if isinstance(node, (ast.Constant, ast.JoinedStr, ast.Dict, ast.Set, ast.BinOp, ast.Compare)):
            return FOREIGN
        return None

    def _library_class(self, dotted: str) -> Kind | None:
        """The kind of a library's name: its class deriving from
        matplotlib's, or something foreign."""
        resolved = self.census.resolve(dotted)
        if resolved and not isinstance(resolved[0], type):
            ancestors = self.census.matplotlib_ancestors(*resolved[0])
            return Kind("class", tuple(ancestors)) if ancestors else FOREIGN
        return FOREIGN if dotted.split(".")[0] not in ("corpus",) else None

    def _member(self, owner: Kind | None, name: str) -> Kind | None:
        if owner is None:
            return None
        if owner.what == "module":
            return self.typer.of_dotted(f"{owner.module}.{name}")
        if owner.what == "sequence" and name in ("flat", "T"):
            return owner
        if owner.what in ("instance", "class"):
            return self.typer.returned(name, owner) if owner.classes else None
        return FOREIGN if owner.what == "foreign" else None

    def _called(self, node: ast.Call) -> Kind | None:
        function = node.func
        if isinstance(function, ast.Name) and function.id == "super" and not self.scope.lookup("super")[0]:
            return next((kind for kind in self.self_names.values() if kind is not None), None)
        if isinstance(function, ast.Name) and function.id == "type" and len(node.args) == 1:
            kind = self.kind(node.args[0])
            return Kind("class", kind.classes) if kind and kind.what == "instance" else None
        if isinstance(function, ast.Attribute):
            owner = self.kind(function.value)
            if owner is None:
                return None
            if owner.what == "sequence" and function.attr in ("ravel", "flatten", "reshape", "squeeze",
                                                              "tolist", "transpose", "copy"):
                return owner
            if owner.what == "mapping" and function.attr == "values":
                return Kind("sequence", inner=owner.inner)
            if owner.what == "module":
                kind = self.kind(function)
                return self._call_of(kind) if kind is not None else None
            if owner.what in ("instance", "class"):
                return self.typer.returned(function.attr, owner)
            return FOREIGN if owner.what == "foreign" else None
        kind = self.kind(function)
        return self._call_of(kind) if kind is not None else None

    def _call_of(self, kind: Kind) -> Kind | None:
        if kind.what == "class":
            return Kind.instance(*kind.classes) if kind.classes else None
        if kind.what == "function":
            module, _, name = kind.module.rpartition(".")
            return self.typer.returned(name, Kind("module", module=module))
        return FOREIGN if kind.what == "foreign" else None
