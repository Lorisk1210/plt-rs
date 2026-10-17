"""Save a representative subset of the figure cases as SVG, PDF and PostScript
and compare with (or write) the vector goldens next to their PNG goldens.

The subset covers lines, markers, dashes, alpha, text and mathtext, images,
clipping, hatches, Gouraud shading, polar axes and collections. plt-rs
compares its own documents with these structurally (plt-spec's
``figure_vectors`` test); keep the list in sync with ``VECTOR_CASES`` there.
"""

from __future__ import annotations

import importlib
import pathlib

import pytest

from spec_support import all_cases

for module in sorted(pathlib.Path(__file__).parent.glob("cases_*.py")):
    importlib.import_module(f"figures.{module.stem}")

VECTOR_CASES = [
    "axes/grid_major_minor_styles",
    "axes/mathtext_labels",
    "bars/hatches",
    "basic/line",
    "contour/tripcolor_gouraud",
    "images/clip_path_circle",
    "images/imshow_default",
    "images/pcolormesh_gouraud",
    "lines/alpha_overlap",
    "lines/joinstyles",
    "lines/linestyles_dash_tuples",
    "lines/markers_filled",
    "patches/clip_path_artists",
    "patches/hatches_basic",
    "patches/linecollection_styles",
    "patches/patchcollection_cmap",
    "polar/fill_and_fill_between",
    "polar/plot_default",
    "scatter/default",
    "scatter/edgecolor_face_and_alpha",
    "stats/fill_between_two_curves_alpha",
    "text/color_alpha_background",
    "text/mathtext_mixed_sizes",
    "text/rotation_default_mode",
    "text/unicode",
]

FORMATS = ["svg", "pdf", "ps"]

_BY_NAME = {case.name: case for case in all_cases()}


@pytest.mark.parametrize("fmt", FORMATS)
@pytest.mark.parametrize("name", VECTOR_CASES)
def test_vector_case(name, fmt, update_goldens):
    case = _BY_NAME[name]
    document = case.render_vector(fmt)
    path = case.vector_path(fmt)
    if update_goldens:
        path.write_bytes(document)
        return
    assert path.exists(), f"missing {fmt} golden for {name}; run --update-goldens"
    assert path.read_bytes() == document
