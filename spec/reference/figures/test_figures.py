"""Render every registered figure case and compare with (or write) goldens."""

from __future__ import annotations

import importlib
import io
import json
import pathlib

import numpy as np
import pytest
from PIL import Image

from spec_support import all_cases
from spec_support.golden import PLATFORM, dump_json, golden_path, record_golden
from spec_support.tex import SKIP_REASON, has_usetex

for module in sorted(pathlib.Path(__file__).parent.glob("cases_*.py")):
    importlib.import_module(f"figures.{module.stem}")


def _pixels(png: bytes) -> np.ndarray:
    return np.asarray(Image.open(io.BytesIO(png)).convert("RGBA"))


#: The largest RMS between pltrs' image of a case and the golden one that
#: ``pltrs-spec`` accepts (``MAX_RMS`` of its ``figure_pixels`` test): keep
#: in step.
MAX_RMS = 1.0


def _rms(golden: bytes, png: bytes) -> float:
    """matplotlib's ``compare_images`` measure of two images, as
    ``pltrs-spec`` takes it: over the colour channels, and over alpha too
    unless both images are opaque; infinite between images of two sizes."""
    expected, actual = _pixels(golden), _pixels(png)
    if expected.shape != actual.shape:
        return np.inf
    if (expected[..., 3] == 255).all() and (actual[..., 3] == 255).all():
        expected, actual = expected[..., :3], actual[..., :3]
    return float(np.sqrt(np.mean((expected.astype(float) - actual.astype(float)) ** 2)))


def _pixels_pass(golden: bytes, png: bytes) -> bool:
    return _rms(golden, png) <= MAX_RMS


def _same_json(golden: bytes, document: bytes) -> bool:
    return json.loads(golden) == json.loads(document)


@pytest.mark.parametrize("case", all_cases(), ids=lambda c: c.name)
def test_figure_case(case, update_goldens):
    if case.needs_tex and not has_usetex():
        pytest.skip(SKIP_REASON)
    png, facts = case.render()
    if update_goldens:
        record_golden(case.png_golden, png, passes=_pixels_pass)
        record_golden(case.facts_golden, dump_json(facts).encode("utf-8"), passes=_same_json)
        return
    golden_png = golden_path(case.png_golden)
    assert golden_png.exists(), f"missing golden for {case.name}; run --update-goldens"
    golden_facts = json.loads(golden_path(case.facts_golden).read_text(encoding="utf-8"))
    assert golden_facts == json.loads(dump_json(facts))
    if PLATFORM is None:
        np.testing.assert_array_equal(_pixels(golden_png.read_bytes()), _pixels(png))
    else:
        assert _pixels_pass(golden_png.read_bytes(), png)
