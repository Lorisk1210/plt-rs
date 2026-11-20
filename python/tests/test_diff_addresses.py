"""
The differential tester's blanking of memory addresses
(``spec/diff/describe.py``, ``Names.text``): whole addresses, in either
case, and the rest of a Windows address that a program blanked with a
lowercase pattern (written2's ``funcanimation_frames_generators``).
"""

import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]


def _names():
    spec = importlib.util.spec_from_file_location("diff_describe", ROOT / "spec/diff/describe.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["diff_describe"] = module
    try:
        spec.loader.exec_module(module)
    finally:
        sys.modules.pop("diff_describe", None)
    return module.Names([], [])


def test_addresses_are_blanked_in_either_case():
    names = _names()
    assert names.text("<f at 0x7f12ab34cd90>") == "<f at 0x?>"
    assert names.text("<f at 0x000001BA38FFD7A0>") == "<f at 0x?>"


def test_the_rest_of_a_windows_address_a_program_blanked_is_blanked():
    # re.sub(r"0x[0-9a-f]+", "0x?", ...) stops at Windows' first capital.
    assert _names().text("<function gen at 0x?BA38FFD7A0> which") == "<function gen at 0x?> which"


def test_short_hex_numbers_stay():
    names = _names()
    assert names.text("flags 0x1F") == "flags 0x1F"
    assert names.text("0x?AB") == "0x?AB"
