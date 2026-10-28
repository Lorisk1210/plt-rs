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
from spec_support.golden import dump_json
from spec_support.tex import SKIP_REASON, has_usetex

for module in sorted(pathlib.Path(__file__).parent.glob("cases_*.py")):
    importlib.import_module(f"figures.{module.stem}")


def _pixels(png: bytes) -> np.ndarray:
    return np.asarray(Image.open(io.BytesIO(png)).convert("RGBA"))


@pytest.mark.parametrize("case", all_cases(), ids=lambda c: c.name)
def test_figure_case(case, update_goldens):
    if case.needs_tex and not has_usetex():
        pytest.skip(SKIP_REASON)
    png, facts = case.render()
    if update_goldens:
        case.png_path.parent.mkdir(parents=True, exist_ok=True)
        case.png_path.write_bytes(png)
        case.facts_path.write_text(dump_json(facts))
        return
    assert case.png_path.exists(), f"missing golden for {case.name}; run --update-goldens"
    assert json.loads(case.facts_path.read_text()) == json.loads(dump_json(facts))
    np.testing.assert_array_equal(_pixels(case.png_path.read_bytes()), _pixels(png))
