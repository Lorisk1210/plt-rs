"""
What a matplotlib-like package does in an environment holding only the
package and what its metadata requires (no pytest, no optional extras),
run by ``check_dist.py --install`` under pltrs and under matplotlib, so
that the two can be compared:

    python -I bare_imports.py pltrs|matplotlib OUTPUT

It writes a JSON object to OUTPUT, of what it ran (``null`` where all
went well, else the exception raised, as ``[type name, message]``, and
the package not found for a `ModuleNotFoundError`):

* ``modules``: the import of each module of the package, under
  matplotlib's names (``matplotlib.*``, ``mpl_toolkits.*``, ``pylab``;
  pltrs' ``pltrs.mpl_toolkits`` and ``pltrs.pylab`` are the latter two);
* ``backends``: a short program per non-interactive backend matplotlib
  names (which needs no display), drawing a figure and saving it in the
  backend's own format;

and of what it read:

* ``imports``: each package outside the standard library and the
  package itself that an ``import`` statement of the package's sources
  names, anywhere (also in a function, which runs only when called), with
  the first module naming it (under its own name);
* ``required``: the packages the requirements of the package's metadata
  provide (their import names).

Run with ``-I``, from outside a source tree, so that only the
environment's packages are found.
"""

import ast
import importlib
import importlib.machinery
import importlib.metadata
import importlib.util
import io
import json
import pathlib
import re
import sys
import warnings

#: The non-interactive backends of matplotlib 3.11's registry
#: (``BackendFilter.NON_INTERACTIVE``).
BACKENDS = ("agg", "cairo", "pdf", "pgf", "ps", "svg", "template")

#: The top-level packages each distribution installs.
ROOTS = {"pltrs": ("pltrs",), "matplotlib": ("matplotlib", "mpl_toolkits", "pylab")}

#: The names a package of either distribution goes by.
OWN = {"pltrs", "matplotlib", "mpl_toolkits", "pylab"}


def outcome(action):
    """None if *action* (a callable) succeeds, else the exception it
    raised as ``[type name, message]``, with the top-level package not
    found after them for a `ModuleNotFoundError`."""
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            action()
    except ModuleNotFoundError as error:
        return [type(error).__qualname__, str(error)[:300], (error.name or "").partition(".")[0]]
    except Exception as error:  # noqa: BLE001 - every failure is the result
        return [type(error).__qualname__, str(error)[:300]]
    return None


def module_files(package):
    """The dotted names of the modules found below the installed
    *package* (its ``.py`` files and extension modules), with their
    files."""
    spec = importlib.util.find_spec(package)
    if spec is None:
        return {}
    if spec.submodule_search_locations is None:
        return {package: pathlib.Path(spec.origin)}
    suffixes = (".py", *importlib.machinery.EXTENSION_SUFFIXES)
    modules = {}
    for base in map(pathlib.Path, spec.submodule_search_locations):
        for path in sorted(base.rglob("*")):
            suffix = next((s for s in suffixes if path.name.endswith(s)), None)
            if suffix is None:
                continue
            directories = path.relative_to(base).parents
            parts = [*path.relative_to(base).parts[:-1], path.name[:-len(suffix)]]
            if parts[-1] == "__init__":
                parts.pop()
            # A directory without __init__ is no package of it (pltrs'
            # backends add theirs to their path on one system only).
            packaged = all((base / directory / "__init__.py").is_file()
                           for directory in directories if directory != pathlib.Path("."))
            if packaged and all(part.isidentifier() for part in parts):
                modules[".".join([package, *parts])] = path
    return modules


def matplotlib_name(name, distribution):
    """The name matplotlib gives the module *name* of *distribution*."""
    if distribution == "matplotlib":
        return name
    for inner, outer in (("pltrs.mpl_toolkits", "mpl_toolkits"), ("pltrs.pylab", "pylab")):
        if name == inner or name.startswith(inner + "."):
            return outer + name[len(inner):]
    return "matplotlib" + name[len("pltrs"):]


def imported_packages(path):
    """The top-level packages the ``import`` statements of the Python
    source *path* name, but those of the standard library and the
    package's own."""
    tree = ast.parse(path.read_bytes(), str(path))
    names = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names.update(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.level == 0:
            names.add(node.module)
    tops = {name.partition(".")[0] for name in names}
    return tops - OWN - set(sys.stdlib_module_names)


def canonical(name):
    """The distribution name *name* as PEP 503 normalises it."""
    return re.sub(r"[-_.]+", "-", name).lower()


def required_packages(distribution):
    """The import names of the packages the requirements of the installed
    *distribution* provide (those that need no extra)."""
    required = set()
    for requirement in importlib.metadata.requires(distribution) or []:
        name, _, marker = requirement.partition(";")
        if "extra" not in marker:
            required.add(canonical(re.match(r"[A-Za-z0-9._-]+", name.strip()).group()))
    return sorted(top for top, owners in importlib.metadata.packages_distributions().items()
                  if any(canonical(owner) in required for owner in owners))


def draws_with(distribution, backend):
    """Draws a figure with *backend* and saves it in the backend's own
    format (the pyplot of *distribution* doing it all)."""
    pyplot = importlib.import_module(f"{distribution}.pyplot")
    pyplot.switch_backend(backend)
    fig, ax = pyplot.subplots()
    ax.plot([1, 3, 2], label="line")
    ax.set_title(r"$\alpha^2$")
    ax.legend()
    buffer = io.BytesIO()
    fig.savefig(buffer, format=fig.canvas.get_default_filetype())
    pyplot.close(fig)
    if not buffer.getvalue():
        raise RuntimeError(f"the {backend} backend wrote nothing")


def main(distribution, output):
    """Writes the tables of *distribution* (``pltrs`` or ``matplotlib``)
    to the file *output*."""
    files = {name: path for root in ROOTS[distribution]
             for name, path in module_files(root).items()}
    imports = {}
    for name, path in sorted(files.items()):
        if path.suffix == ".py":
            for package in imported_packages(path):
                imports.setdefault(package, name)
    modules = {matplotlib_name(name, distribution):
               outcome(lambda name=name: importlib.import_module(name))
               for name in sorted(files)}
    backends = {backend: outcome(lambda backend=backend: draws_with(distribution, backend))
                for backend in BACKENDS}
    tables = {"modules": modules, "backends": backends, "imports": imports,
              "required": required_packages(distribution)}
    with open(output, "w", encoding="utf-8") as file:
        json.dump(tables, file, indent=1, sort_keys=True)


if __name__ == "__main__":
    main(*sys.argv[1:])
