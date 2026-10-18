"""Save a representative subset of the figure cases as SVG, PDF and PostScript
and compare with (or write) the vector goldens next to their PNG goldens.

The subset covers lines, markers, dashes, alpha, text and mathtext, text in
the standard PostScript fonts (``pdf.use14corefonts``, ``ps.useafm``),
images, clipping, hatches, Gouraud shading, polar axes, collections,
rasterized artists and colorbars. Some cases are saved again with other font settings
(Type 42 fonts, SVG text as text), and two cases are collected into one
multi-page PDF. plt-rs compares its own documents with these structurally
(plt-spec's ``figure_vectors`` test); keep the lists in sync with
``VECTOR_CASES``, ``VECTOR_VARIANTS`` and ``PDF_PAGES`` there.
"""

from __future__ import annotations

import dataclasses
import importlib
import io
import os
import pathlib

import pytest

from spec_support import all_cases
from spec_support.figcases import FIGURE_ROOT, VECTOR_METADATA

for module in sorted(pathlib.Path(__file__).parent.glob("cases_*.py")):
    importlib.import_module(f"figures.{module.stem}")

VECTOR_CASES = [
    "afm/core_fonts_layout",
    "afm/core_fonts_text",
    "afm/ps_fonts_layout",
    "afm/ps_fonts_text",
    "axes/grid_major_minor_styles",
    "axes/mathtext_labels",
    "bars/hatches",
    "basic/line",
    "contour/tripcolor_gouraud",
    "hinting/axes_100dpi",
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
    "vectors/colorbar_many_colors",
    "vectors/rasterized_artists",
    "vectors/rasterization_zorder",
    "vectors/agg_filter",
    "vectors/text_fonts",
]

FORMATS = ["svg", "pdf", "ps"]

#: Cases saved again with other settings: case, variant name (part of the
#: golden's file name), rcParams and formats.
VECTOR_VARIANTS = [
    ("vectors/text_fonts", "type42", {"pdf.fonttype": 42, "ps.fonttype": 42}, ["pdf", "ps"]),
    ("vectors/text_fonts", "nofont", {"svg.fonttype": "none"}, ["svg"]),
]

#: Cases collected, one per page, into ``vectors/pages.pdf``; a note is
#: attached to the second page.
PDF_PAGES = ["vectors/text_fonts", "vectors/rasterized_artists"]

_BY_NAME = {case.name: case for case in all_cases()}


def _check(path, document, update_goldens):
    if update_goldens:
        path.write_bytes(document)
        return
    assert path.exists(), f"missing golden {path}; run --update-goldens"
    assert path.read_bytes() == document


@pytest.mark.parametrize("fmt", FORMATS)
@pytest.mark.parametrize("name", VECTOR_CASES)
def test_vector_case(name, fmt, update_goldens):
    case = _BY_NAME[name]
    _check(case.vector_path(fmt), case.render_vector(fmt), update_goldens)


@pytest.mark.parametrize(
    "name,variant,rc,fmt",
    [(n, v, rc, f) for n, v, rc, formats in VECTOR_VARIANTS for f in formats],
)
def test_vector_variant(name, variant, rc, fmt, update_goldens):
    case = _BY_NAME[name]
    varied = dataclasses.replace(case, rc={**case.rc, **rc})
    path = FIGURE_ROOT / f"{name}.{variant}.{fmt}"
    _check(path, varied.render_vector(fmt), update_goldens)


def test_pdf_pages(update_goldens):
    import matplotlib as mpl
    import matplotlib.pyplot as plt
    from matplotlib.backends.backend_pdf import PdfPages

    previous_epoch = os.environ.get("SOURCE_DATE_EPOCH")
    os.environ["SOURCE_DATE_EPOCH"] = "0"
    buffer = io.BytesIO()
    try:
        with PdfPages(buffer, metadata={**VECTOR_METADATA["pdf"], "Title": "Pages"}) as pages:
            for page, name in enumerate(PDF_PAGES):
                case = _BY_NAME[name]
                if page == 1:
                    pages.attach_note("the second page")
                with mpl.rc_context(case.rc):
                    fig = plt.figure(figsize=case.figsize, dpi=case.dpi)
                    try:
                        case.build(fig)
                        pages.savefig(fig, dpi=case.dpi)
                    finally:
                        plt.close(fig)
    finally:
        if previous_epoch is None:
            del os.environ["SOURCE_DATE_EPOCH"]
        else:
            os.environ["SOURCE_DATE_EPOCH"] = previous_epoch
    _check(FIGURE_ROOT / "vectors" / "pages.pdf", buffer.getvalue(), update_goldens)
