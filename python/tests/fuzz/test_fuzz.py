"""The fuzzer as a test: no program meets a panic, a crash, a hang or
runaway memory (see the package's docstring for the settings)."""

import os

from . import runner


def test_programs_meet_no_panic_crash_hang_or_runaway_memory():
    bugs = runner.run(scale=int(os.environ.get("PLTRS_FUZZ_SCALE", "1")),
                      replay=os.environ.get("PLTRS_FUZZ_REPLAY"),
                      jobs=int(os.environ.get("PLTRS_FUZZ_JOBS", "4")),
                      first=int(os.environ.get("PLTRS_FUZZ_FIRST", "0")))
    assert not runner.real(bugs), runner.summary(bugs)
