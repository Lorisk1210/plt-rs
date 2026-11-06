"""
Smoke tests of the PGF backend as matplotlib's measures and writes: text
measured by the TeX system (`RendererPgf.get_text_width_height_descent`,
its errors and the unknown fonts it logs), images written beside a named
file with their rows from the top, and the refusal of images in a stream.
The expected values were recorded from matplotlib 3.11.2 running the same
code; the texts measured are rules, whose size no font changes.
"""

import io
import logging
import shutil

import numpy as np
import pytest
from PIL import Image

import pltrs
import pltrs.pyplot as plt
from pltrs.backends import backend_pgf
from pltrs.font_manager import FontProperties

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

needs_xelatex = pytest.mark.skipif(not shutil.which("xelatex"),
                                   reason="XeLaTeX is not installed")


@needs_xelatex
def test_text_is_measured_by_latex_in_points_of_a_72nd_inch():
    renderer = backend_pgf.RendererPgf(plt.figure(dpi=72), None)
    rule = r"\rule{36pt}{9pt}"
    assert renderer.get_text_width_height_descent(rule, FontProperties(), False) == (
        36.0, 9.0, 0.0)
    lowered = r"\raisebox{-3pt}{\rule{36pt}{9pt}}"
    assert renderer.get_text_width_height_descent(lowered, FontProperties(), True) == (
        36.0, 9.0, 3.0)
    renderer.dpi = 144
    assert renderer.get_text_width_height_descent(rule, FontProperties(), "TeX") == (
        72.0, 18.0, 0.0)


@needs_xelatex
def test_texts_latex_cannot_typeset_raise_value_error():
    renderer = backend_pgf.RendererPgf(plt.figure(dpi=72), None)
    with pytest.raises(ValueError, match=r"^Error measuring \{\\sffamily\\fontsize"):
        renderer.get_text_width_height_descent(r"\oops", FontProperties(), False)
    assert renderer.get_text_width_height_descent(
        r"\rule{1pt}{1pt}", FontProperties(), False) == (1.0, 1.0, 0.0)


@needs_xelatex
def test_unknown_fonts_are_logged_when_measured_and_drawn(caplog):
    with caplog.at_level("WARNING"), pltrs.rc_context(
            {"font.family": "this-font-does-not-exist"}):
        fig = plt.figure()
        fig.text(.5, .5, "hello")
        fig.savefig(io.BytesIO(), format="pgf")
    ignored = [(record.name, record.getMessage()) for record in caplog.records
               if record.getMessage().startswith("Ignoring")]
    assert ignored == [("pltrs.backends.backend_pgf",
                        "Ignoring unknown font: this-font-does-not-exist")] * 2


@needs_xelatex
def test_images_go_beside_named_files_rows_from_the_top(tmp_path):
    fig, ax = plt.subplots()
    ax.imshow(np.array([[[1.0, 0, 0], [0, 1, 0]], [[0, 0, 1], [1, 1, 1]]]))
    fig.savefig(tmp_path / "picture.pgf", backend="pgf")
    assert sorted(p.name for p in tmp_path.iterdir()) == [
        "picture-img0.png", "picture.pgf"]
    image = np.asarray(Image.open(tmp_path / "picture-img0.png"))
    assert image.shape == (369, 370, 4)
    assert image[0, 0].tolist() == [255, 0, 0, 255]
    assert image[-1, -1].tolist() == [255, 255, 255, 255]
    included = [line for line in (tmp_path / "picture.pgf").read_text().splitlines()
                if "includegraphics" in line]
    assert included == [
        r"\pgftext[left,bottom]{\includegraphics[interpolate=true,width=3.700000in,"
        r"height=3.690000in]{picture-img0.png}}%"]
    with pytest.raises(ValueError, match="^streamed pgf-code does not support raster "
                       "graphics, consider using the pgf-to-pdf option$"):
        fig.savefig(io.BytesIO(), format="pgf")


@needs_xelatex
def test_drawing_the_canvas_lays_the_figure_out():
    fig = plt.figure(layout="constrained")
    backend_pgf.FigureCanvasPgf(fig)
    ax = fig.add_subplot()
    before = ax.get_position().bounds
    fig.canvas.draw()
    assert ax.get_position().bounds != before


@needs_xelatex
def test_known_families_are_not_logged(caplog):
    with caplog.at_level(logging.WARNING):
        fig = plt.figure()
        fig.text(.5, .5, "hello", family="serif")
        fig.savefig(io.BytesIO(), format="pgf")
    assert not [r for r in caplog.records if r.getMessage().startswith("Ignoring")]
