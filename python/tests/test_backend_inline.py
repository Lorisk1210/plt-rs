"""
Smoke tests of Jupyter's inline backend for pltrs (`pltrs.backends.
backend_inline`): IPython's kernel selects
``module://matplotlib_inline.backend_inline``, which pltrs loads as its own
port unless it is installed as ``matplotlib``; a notebook run headlessly
(when nbclient and ipykernel are installed) shows its figures as images,
as matplotlib's ``test_backend_inline.py`` expects of matplotlib.
"""

import importlib.util
import os

import pytest

from pltrs.backends import backend_registry


def test_the_kernels_backend_is_pltrs_own_port():
    # With the matplotlib distribution installed, matplotlib-inline's own
    # module imports pltrs as matplotlib, and serves.
    aliased = importlib.util.find_spec("_pltrs_matplotlib_alias") is not None
    assert backend_registry._backend_module_name(
        "module://matplotlib_inline.backend_inline") == (
        "matplotlib_inline.backend_inline" if aliased else "pltrs.backends.backend_inline")
    # Other modules stay themselves.
    assert backend_registry._backend_module_name(
        "module://some.backend") == "some.backend"


@pytest.mark.skipif(not all(importlib.util.find_spec(name) for name in
                            ["nbclient", "nbformat", "ipykernel"]),
                    reason="needs nbclient, nbformat and ipykernel")
def test_notebook_shows_figures_inline():
    import nbformat
    from nbclient import NotebookClient

    cells = [
        "import pltrs.pyplot as plt",
        "fig, ax = plt.subplots(figsize=(3, 2))\nax.plot([1, 3, 2])",
        "import pltrs\npltrs.get_backend()",
        "import sys\n[m for m in sys.modules if m.split('.')[0] == 'matplotlib']",
    ]
    nb = nbformat.v4.new_notebook(
        cells=[nbformat.v4.new_code_cell(source) for source in cells])
    os.environ.setdefault("PLTRS_LOOK", "matplotlib")
    NotebookClient(nb, timeout=120, kernel_name="python3").execute()
    errors = [output for cell in nb.cells for output in cell.outputs
              if output.output_type == "error"]
    assert not errors
    image = [output for output in nb.cells[1].outputs
             if output.output_type == "display_data"][0]["data"]
    assert image["text/plain"] == "<Figure size 300x200 with 1 Axes>"
    assert "image/png" in image
    backend = nb.cells[2].outputs[0]["data"]["text/plain"]
    assert backend == "'module://matplotlib_inline.backend_inline'"
    assert nb.cells[3].outputs[0]["data"]["text/plain"] == "[]", \
        "no matplotlib module was imported"
