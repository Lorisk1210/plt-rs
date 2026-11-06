"""matplotlib's public callables, as ``cargo xtask rust-coverage`` counts
them: the list ``cargo xtask diff --coverage`` measures its programs on
(``spec/diff/api_index.py``: the modules and classes of
``spec/api-audit/enumerate.py``; their functions, classes, methods,
properties, class and static methods, and the members of the private bases
of public classes).

Run in the reference venv, it prints one callable per line, tab-separated:
its key (``module:Owner.name``, ``module:name`` at module level), its kind
and ``deprecated`` for those matplotlib deprecates (empty otherwise).
"""

from __future__ import annotations

import pathlib
import sys
import warnings

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "diff"))
import api_index  # noqa: E402


def main() -> int:
    """Print the callables."""
    records = api_index.enumerate_api(sys.executable)
    import matplotlib

    matplotlib.use("agg")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        index = api_index.Index(records)
    for callable_ in index.callables:
        print(f"{callable_.key}\t{callable_.kind}\t{'deprecated' if callable_.deprecated else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
