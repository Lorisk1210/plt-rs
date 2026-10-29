"""Runs the Python examples under real matplotlib, for `cargo xtask examples`.

    python run_matplotlib.py OUT_DIR SCRIPT...

The examples are written for pltrs, plt-rs' Python package with matplotlib's
syntax, and import it (`import pltrs.pyplot as plt`). Here every `pltrs`
import is answered with the matplotlib module of the same name, and figures
are drawn in plt-rs' modern look, the look pltrs draws in by default: the
`plt-modern` style sheet with Inter, set up for matplotlib as the spec's
`cases_modern.py` does it. Each script runs in OUT_DIR, where it saves its
figure. A script that fails shows its traceback on stderr and its name on
stdout, and the others still run.
"""

import importlib
import importlib.abc
import importlib.util
import os
import pathlib
import runpy
import sys
import traceback

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parents[3]
sys.path[:0] = [str(REPO / "spec" / "reference"), str(REPO / "spec" / "reference" / "figures")]
# Adds Inter to the font manager and emulates the sheet's math symbols.
import cases_modern  # noqa: E402


class PltrsIsMatplotlib(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    """Imports `pltrs` and `pltrs.<module>` as `matplotlib` and
    `matplotlib.<module>`: the very module objects, already run."""

    def find_spec(self, name, path=None, target=None):
        if name == "pltrs" or name.startswith("pltrs."):
            return importlib.util.spec_from_loader(name, self)
        return None

    def create_module(self, spec):
        module = importlib.import_module("matplotlib" + spec.name.removeprefix("pltrs"))
        self._spec = module.__spec__
        return module

    def exec_module(self, module):
        # The import machinery gave the module the alias's spec: give it its
        # own back.
        module.__spec__ = self._spec


def modern_look():
    """matplotlib's defaults under the modern look's style sheet."""
    matplotlib.rcdefaults()
    matplotlib.rcParams.update(cases_modern.MODERN)


def run(script, out):
    """Runs one example in `out`; returns whether it succeeded."""
    script = pathlib.Path(script).resolve()
    modern_look()
    sys.path.insert(0, str(script.parent))
    cwd = os.getcwd()
    os.chdir(out)
    try:
        runpy.run_path(str(script), run_name="__main__")
        return True
    except Exception:
        traceback.print_exc()
        return False
    finally:
        os.chdir(cwd)
        sys.path.remove(str(script.parent))
        plt.close("all")


def main():
    out, scripts = pathlib.Path(sys.argv[1]), sys.argv[2:]
    out.mkdir(parents=True, exist_ok=True)
    sys.meta_path.insert(0, PltrsIsMatplotlib())
    failed = [script for script in scripts if not run(script, out)]
    for script in failed:
        print(pathlib.Path(script).stem)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
