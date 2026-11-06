"""
What ``import pltrs.pyplot`` loads, against what ``import
matplotlib.pyplot`` loads: modules matplotlib leaves for later stay
unimported until a program uses them.
"""

import os
import subprocess
import sys

# A fresh interpreter importing pyplot, then naming the modules asked for
# that are loaded.
PROGRAM = """
import sys
import pltrs.pyplot
print(",".join(sorted(name for name in sys.argv[1:] if name in sys.modules)))
"""


def loaded_after_import(*modules):
    """Which of `modules` a fresh ``import pltrs.pyplot`` loads."""
    env = dict(os.environ, MPLBACKEND="agg")
    result = subprocess.run(
        [sys.executable, "-c", PROGRAM, *modules],
        env=env, capture_output=True, text=True, check=True)
    return [name for name in result.stdout.strip().split(",") if name]


def test_pyplot_leaves_zoneinfo_unimported():
    # matplotlib 3.11.2's pyplot imports dateutil but not zoneinfo, whose
    # search path reads the interpreter's build settings (sysconfig).
    assert loaded_after_import("zoneinfo", "dateutil") == ["dateutil"]

