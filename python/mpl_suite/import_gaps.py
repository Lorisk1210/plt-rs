"""
Lists what matplotlib's test files import at module level that pltrs
lacks: every gap of a file, where a collection error shows only the first.

``cargo xtask mpl-suite --imports`` runs it in the pltrs site (where
``matplotlib`` is pltrs) with the reference outcome file as argument. It
prints, for each test file with tests passing in the reference run, the
modules and names its top-level ``import`` statements need but cannot
get; then each missing module or name with the reference-passing tests of
the files needing it, the most first. A file's tests can only run once all
of its gaps are filled, so these counts are what each gap holds back, not
what filling it alone unblocks.
"""

import ast
import collections
import importlib
import json
import pathlib
import sys

#: The packages whose names pltrs provides.
_PACKAGES = ("matplotlib", "mpl_toolkits", "pylab")


def _needs(path):
    """The (module, name) pairs the top-level imports of *path* need."""
    tree = ast.parse(path.read_text(encoding="utf-8"))
    for node in tree.body:
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name.split(".")[0] in _PACKAGES:
                    yield alias.name, None
        elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
            if node.module.split(".")[0] in _PACKAGES:
                for alias in node.names:
                    yield node.module, alias.name


def _missing(module, name):
    """What of ``from module import name`` cannot be imported, if anything."""
    try:
        imported = importlib.import_module(module)
    except Exception:  # noqa: BLE001 - any failure is a gap
        return module
    if name is None or name == "*" or hasattr(imported, name):
        return None
    try:
        importlib.import_module(f"{module}.{name}")
    except Exception:  # noqa: BLE001
        return f"{module}.{name}"
    return None


def main(reference_path):
    outcomes = json.loads(pathlib.Path(reference_path).read_text())["outcomes"]
    passing = collections.Counter(
        test.split("::")[0] for test, outcome in outcomes.items() if outcome == "passed")
    held_back = collections.Counter()
    for file in sorted(passing):
        gaps = sorted({gap for module, name in _needs(pathlib.Path(file))
                       if (gap := _missing(module, name))})
        if gaps:
            print(f"{file} ({passing[file]} passing in the reference run):")
            for gap in gaps:
                print(f"    {gap}")
                held_back[gap] += passing[file]
    print("\nmissing modules and names, by the reference-passing tests of the "
          "files that import them:")
    for gap, count in held_back.most_common():
        print(f"{count:>6}  {gap}")


if __name__ == "__main__":
    main(sys.argv[1])
