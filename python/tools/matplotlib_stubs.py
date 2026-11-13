"""
Keeps pltrs' type information matplotlib's: writes matplotlib's ``py.typed``
and type stubs (``lib/matplotlib/**/*.pyi`` of its source tree) beside the
modules they describe, for both distributions:

    python python/tools/matplotlib_stubs.py [--sdist DIR] [--check]

* into ``python/alias/matplotlib``, for the ``matplotlib`` distribution
  (the alias), each stub as matplotlib installs it;
* into ``python/pltrs``, for the ``pltrs`` distribution, each stub with
  its imports of matplotlib's modules importing pltrs' instead
  (``from matplotlib.text import Text`` is ``from pltrs.text import Text``).
  Both keep matplotlib's form, absolute imports, which type checkers read
  otherwise than relative ones: pyright takes ``from . import x`` in a stub
  for a re-export, and a package's relative imports in its ``__init__``
  for importing those submodules (``mpl.colors`` would be known after
  ``import matplotlib as mpl``, which pyright reports unknown for
  matplotlib). Imports of ``mpl_toolkits`` stay as they are, so that the
  toolkits are as untyped as matplotlib's;
* a stub of each module matplotlib types inline rather than in a stub
  (`INLINE_TYPED`: ``pyplot``), whose pltrs module carries no annotations:
  matplotlib's module without docstrings and the bodies of its functions,
  which is what mypy reads of it (it infers no return type from a body),
  and with every public name it binds in ``__all__``, as mypy exports a
  module's imports (``plt.Figure``, ``plt.np``) but a stub's only where it
  says so. (pyright takes a typed module's imports for private names whose
  use it reports, which no stub can say: under pltrs it does not report
  them);
* into ``python/alias/matplotlib`` too, a stub of each module matplotlib
  installs without one (``units``, ``dates``, the backends), as the alias
  installs stubs, not code, for type checkers: its code without
  docstrings and function bodies but for what methods assign on their
  instance (the attributes a checker reads of a class), with its
  ``__all__`` or one of every public name it binds; and into
  ``python/alias/mpl_toolkits``, beside each toolkit's loader, the stub of
  matplotlib's ``__init__`` of the toolkit (pyright reads an untyped
  package's code, which the loader would leave empty; the toolkits have
  no ``py.typed``, as matplotlib's);
* only the stubs matplotlib installs (those its ``meson.build`` lists:
  3.11.2's source tree also has ``dates.pyi`` and ``backends/registry.pyi``,
  which its wheels leave out, so that type checkers read those modules'
  code), none for a module pltrs does not have (`skipped`: matplotlib's C++
  ``_qhull`` and the macOS backend's ``_macosx``), and none for pltrs' own
  modules.

``--sdist`` is matplotlib's source tree (default: ``MPL_SDIST``, else
``target/refs/matplotlib-dl/matplotlib-<version>``, the version
``python/pltrs/__init__.py`` implements). ``--check`` writes nothing and
exits with 1 if a file differs from what it would write. After a new
matplotlib, run it, then ``cargo xtask diff --types``.
"""

import argparse
import ast
import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
#: Where each distribution's stubs go, by the name its package has.
TREES = {"pltrs": ROOT / "python" / "pltrs", "matplotlib": ROOT / "python" / "alias" / "matplotlib",
         "mpl_toolkits": ROOT / "python" / "alias" / "mpl_toolkits"}
#: The trees typed by ``py.typed`` (matplotlib's toolkits are not).
TYPED = ("pltrs", "matplotlib")

#: matplotlib's modules typed inline rather than in a stub, whose stubs
#: are their code without function bodies.
INLINE_TYPED = ("pyplot",)

#: An import of matplotlib's modules at a line's start: ``from
#: matplotlib[.x] import`` or ``import matplotlib.x[.y] as z``.
IMPORT = re.compile(r"^(\s*)(from matplotlib(?:\.\w+)* import\b"
                    r"|import matplotlib(?:\.\w+)+ as \w+\s*$)")
#: Any other import of matplotlib's modules, which no stub has.
OTHER_IMPORT = re.compile(r"^\s*import matplotlib\b")


def matplotlib_version():
    """The matplotlib version pltrs implements (its ``__version__``)."""
    init = (TREES["pltrs"] / "__init__.py").read_text(encoding="utf-8")
    return re.search(r'^__version__ = "([^"]+)"$', init, re.MULTILINE).group(1)


def header(module, how):
    """The comment opening the stub of *module*, saying where it is from."""
    return (f"# The type stub of {module}: matplotlib {matplotlib_version()}'s, under the\n"
            f"# Matplotlib License, {how}\n# (python/tools/matplotlib_stubs.py).\n\n")


def pltrs_imports(text, where):
    """*text*, a stub, importing pltrs' modules where it imports
    matplotlib's. Fails on an import it cannot rewrite."""
    lines = []
    for line in text.splitlines(keepends=True):
        found = IMPORT.match(line)
        if found:
            renamed = re.sub(r"\bmatplotlib\b", "pltrs", found.group(2), count=1)
            line = line[:found.start(2)] + renamed + line[found.end(2):]
        elif OTHER_IMPORT.match(line):
            raise ValueError(f"{where}: cannot rewrite this import: {line.strip()}")
        lines.append(line)
    return "".join(lines)


class _BodiesRemoved(ast.NodeTransformer):
    """A module's code as a stub: docstrings and function bodies removed,
    and matplotlib's package named *package*."""

    def __init__(self, package):
        self.package = package
        #: Whether the functions visited are methods.
        self.in_class = False

    def visit_Module(self, node):
        self.generic_visit(node)
        node.body = _without_docstring(node.body)
        bound = list(dict.fromkeys(_bound(node.body)))
        if "__all__" not in bound:
            # A module's imports are its names for type checkers (mypy's
            # implicit re-export), a stub's only where it says so: all of
            # them.
            exported = [ast.Constant(name) for name in bound if not name.startswith("_")]
            node.body.append(ast.Assign([ast.Name("__all__", ast.Store())],
                                        ast.List(exported, ast.Load())))
        return node

    def visit_FunctionDef(self, node):
        node.decorator_list = [self.visit(decorator) for decorator in node.decorator_list]
        node.args = self.visit(node.args)
        if node.returns is not None:
            node.returns = self.visit(node.returns)
        # A method keeps what it assigns on its instance: the attributes a
        # checker reads of the class.
        stores = [self.visit(statement) for statement in _instance_stores(node)] \
            if self.in_class else []
        node.body = stores or [ast.Expr(ast.Constant(...))]
        return node

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_ClassDef(self, node):
        outer, self.in_class = self.in_class, True
        self.generic_visit(node)
        self.in_class = outer
        node.body = _without_docstring(node.body) or [ast.Expr(ast.Constant(...))]
        return node

    def visit_ImportFrom(self, node):
        if node.level == 0 and node.module is not None:
            node.module = self.renamed(node.module)
        return node

    def visit_Import(self, node):
        for alias in node.names:
            alias.name = self.renamed(alias.name)
        return node

    def visit_Name(self, node):
        if node.id == "matplotlib":
            node.id = self.package
        return node

    def renamed(self, module):
        """*module* named in *package* where it is matplotlib's."""
        head, dot, rest = module.partition(".")
        return f"{self.package}{dot}{rest}" if head == "matplotlib" else module


def _bound(body):
    """The names the statements *body* bind at their level (and in the
    blocks of their ``if`` statements: ``TYPE_CHECKING``), but for
    ``__future__``'s."""
    for statement in body:
        if isinstance(statement, ast.Import):
            yield from (a.asname or a.name.partition(".")[0] for a in statement.names)
        elif isinstance(statement, ast.ImportFrom) and statement.module != "__future__":
            yield from (a.asname or a.name for a in statement.names if a.name != "*")
        elif isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            yield statement.name
        elif isinstance(statement, (ast.Assign, ast.AnnAssign)):
            targets = statement.targets if isinstance(statement, ast.Assign) else [statement.target]
            yield from (t.id for t in targets if isinstance(t, ast.Name))
        elif isinstance(statement, ast.If):
            yield from _bound(statement.body)
            yield from _bound(statement.orelse)


def _instance_stores(function):
    """The statements of *function*'s body (not of the functions it
    defines) assigning attributes of its first parameter (``self.x = ...``)."""
    arguments = function.args.posonlyargs + function.args.args
    if not arguments:
        return []
    owner = arguments[0].arg
    stores, pending = [], list(function.body)
    while pending:
        statement = pending.pop(0)
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            continue
        targets = (statement.targets if isinstance(statement, ast.Assign)
                   else [statement.target] if isinstance(statement, ast.AnnAssign) else [])
        if any(isinstance(item, ast.Attribute) and isinstance(item.value, ast.Name)
               and item.value.id == owner
               for target in targets for item in ast.walk(target)):
            stores.append(statement)
        for field in ("body", "orelse", "finalbody", "handlers"):
            pending.extend(getattr(statement, field, None) or [])
    return stores


def _without_docstring(body):
    """*body* without the docstring it opens with."""
    if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) \
            and isinstance(body[0].value.value, str):
        return body[1:]
    return body


def code_stub(source, package):
    """The stub of the module whose code is *source* (matplotlib's typed
    inline, or not typed), in the package named *package*."""
    tree = _BodiesRemoved(package).visit(ast.parse(source))
    return ast.unparse(ast.fix_missing_locations(tree)) + "\n"


def module_exists(relative_stub):
    """Whether pltrs has the module whose stub is *relative_stub* (a path
    below the package)."""
    base = TREES["pltrs"] / relative_stub.with_suffix("")
    if relative_stub.name == "__init__.pyi":
        return base.with_suffix(".py").exists()
    return base.with_suffix(".py").exists() or (base / "__init__.py").exists()


def stubless(module, path):
    """Whether matplotlib installs the module *module* (*path* below its
    package) without a stub, and pltrs has it: not its tests nor the
    Sphinx extensions."""
    if path.parts[0] in ("tests", "sphinxext") or path.stem in INLINE_TYPED:
        return False
    if module.with_suffix(".pyi").exists() and installed(module.with_suffix(".pyi")):
        return False
    return module_exists(path.with_suffix(".pyi"))


def installed(stub):
    """Whether matplotlib installs *stub*: whether the ``meson.build`` of
    its directory names it."""
    build = stub.parent / "meson.build"
    return build.exists() and f"'{stub.name}'" in build.read_text(encoding="utf-8")


def stubs(sdist):
    """What each distribution's stubs should be, by its package's name and
    then by path below the package, and the stubs of matplotlib's that
    pltrs has no module for."""
    lib = sdist / "lib" / "matplotlib"
    marker = (lib / "py.typed").read_text(encoding="utf-8")
    wanted = {package: {pathlib.Path("py.typed"): marker} if package in TYPED else {}
              for package in TREES}
    skipped = []
    for stub in sorted(lib.rglob("*.pyi")):
        path = stub.relative_to(lib)
        if not installed(stub):
            continue
        if not module_exists(path):
            skipped.append(path)
            continue
        module = ".".join(("matplotlib", *path.with_suffix("").parts)).removesuffix(".__init__")
        text = stub.read_text(encoding="utf-8")
        wanted["matplotlib"][path] = header(module, "as matplotlib installs it") + text
        wanted["pltrs"][path] = header(module, "importing pltrs' modules for matplotlib's") \
            + pltrs_imports(text, path)
    # A checker reads the alias' stubs only: the modules matplotlib types
    # by its code, or not at all, get a stub of that code.
    for module in sorted(lib.rglob("*.py")):
        path = module.relative_to(lib)
        if stubless(module, path):
            name = ".".join(("matplotlib", *path.with_suffix("").parts)).removesuffix(".__init__")
            wanted["matplotlib"][path.with_suffix(".pyi")] = header(
                name, "not typed or typed inline there: its code without docstrings\n# and "
                "function bodies but for what they assign on the instance") + code_stub(
                module.read_text(encoding="utf-8"), "matplotlib")
    # The alias' toolkits are its loaders, which a checker reading their
    # code (pyright does, of an untyped package) would find empty: beside
    # each, the stub of matplotlib's ``__init__`` (pltrs' modules beside
    # it, `Project.checked_sources`).
    for init in sorted((sdist / "lib" / "mpl_toolkits").glob("*/__init__.py")):
        path = init.relative_to(sdist / "lib" / "mpl_toolkits").with_suffix(".pyi")
        if (TREES["mpl_toolkits"] / path).with_suffix(".py").exists():
            wanted["mpl_toolkits"][path] = header(
                f"mpl_toolkits.{path.parent}", "its code without docstrings and function bodies") \
                + code_stub(init.read_text(encoding="utf-8"), "matplotlib")
    for name in INLINE_TYPED:
        path = pathlib.Path(f"{name}.pyi")
        source = (lib / f"{name}.py").read_text(encoding="utf-8")
        for package in TYPED:
            wanted[package][path] = header(
                f"matplotlib.{name}", "typed inline there: its code without docstrings\n# and "
                "function bodies") + code_stub(source, package)
    return wanted, skipped


def present(tree):
    """The stubs and ``py.typed`` in *tree*, by path below it."""
    found = {path.relative_to(tree) for path in tree.rglob("*.pyi")}
    return found | {path.relative_to(tree) for path in tree.glob("py.typed")}


def main(argv):
    """Writes (or checks) the stubs; exits with 1 if a check fails."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    default = os.environ.get("MPL_SDIST") or ROOT / "target" / "refs" / "matplotlib-dl" / \
        f"matplotlib-{matplotlib_version()}"
    parser.add_argument("--sdist", type=pathlib.Path, default=default)
    parser.add_argument("--check", action="store_true")
    options = parser.parse_args(argv)
    wanted, skipped = stubs(options.sdist)
    for path in skipped:
        print(f"no module for matplotlib's stub {path}: skipped")
    differing = 0
    for package, tree in TREES.items():
        files = wanted[package]
        stale = sorted(present(tree) - set(files))
        changed = sorted(path for path, text in files.items()
                         if not (tree / path).exists()
                         or (tree / path).read_text(encoding="utf-8") != text)
        differing += len(stale) + len(changed)
        if options.check:
            for path in changed:
                print(f"differs from matplotlib's: {tree.relative_to(ROOT) / path}")
            for path in stale:
                print(f"not matplotlib's: {tree.relative_to(ROOT) / path}")
            continue
        for path in changed:
            (tree / path).parent.mkdir(parents=True, exist_ok=True)
            (tree / path).write_text(files[path], encoding="utf-8")
        for path in stale:
            (tree / path).unlink()
        print(f"{tree.relative_to(ROOT)}: {len(files)} files, {len(changed)} written, "
              f"{len(stale)} removed")
    return 1 if options.check and differing else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
