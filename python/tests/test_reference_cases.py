"""
The matplotlib-code references of pltrs (Python) run by programs, each
compared with what matplotlib 3.11 made of it (`reference_cases`: the
programs, their fingerprints, and how to record matplotlib's).
"""

import json

import pytest

import pltrs

import reference_cases


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield


GOLDEN = json.loads(reference_cases.GOLDEN.read_text(encoding="utf-8"))


def test_every_program_has_matplotlibs_fingerprint():
    assert sorted(GOLDEN) == sorted(reference_cases.CASES)


@pytest.mark.parametrize("name", sorted(reference_cases.CASES))
def test_a_reference_makes_what_matplotlib_makes(name):
    assert reference_cases.run(name, "pltrs") == GOLDEN[name]
