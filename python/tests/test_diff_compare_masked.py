"""
The differential tester's comparison of values (``spec/diff/compare.py``)
on masked arrays of objects: a masked element reads as None on both
sides, and is not walked as a description.
"""

import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def _compare():
    spec = importlib.util.spec_from_file_location("diff_compare", ROOT / "spec/diff/compare.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["diff_compare"] = module
    try:
        spec.loader.exec_module(module)
    finally:
        sys.modules.pop("diff_compare", None)
    return module


def _object_array(values, mask):
    return {"k": "ndarray", "dtype": "object", "type": "numpy.ma.MaskedArray", "shape": [len(values)],
            "v": values, "mask": mask}


def _date(day):
    return {"k": "str", "v": f"2024-01-0{day}"}


def test_masked_elements_of_an_object_array_are_skipped():
    walk = _compare()._Walk()
    a = _object_array([_date(1), _date(2), _date(3)], [False, True, False])
    found = list(walk.walk(a, dict(a), "x"))
    assert all(kind != "type" for kind, *_ in found)


def test_an_element_masked_on_one_side_reports_the_mask():
    walk = _compare()._Walk()
    a = _object_array([_date(1), _date(2)], [False, True])
    b = _object_array([_date(1), _date(2)], [False, False])
    found = list(walk.walk(a, b, "x"))
    assert ("value", "array mask", "x", "") in found
