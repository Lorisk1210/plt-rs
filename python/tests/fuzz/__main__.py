"""``python -m fuzz``: runs the fuzzer and prints the bugs it found."""

import argparse
import os
import sys

from . import runner


def main():
    """Runs the fuzzer with the command line's settings."""
    parser = argparse.ArgumentParser(prog="python -m fuzz", description=__doc__)
    parser.add_argument("--scale", type=int, default=int(os.environ.get("PLTRS_FUZZ_SCALE", "1")),
                        help="run 24 × SCALE programs")
    parser.add_argument("--first", type=int, default=int(os.environ.get("PLTRS_FUZZ_FIRST", "0")),
                        help="start from this seed (to explore programs not run yet)")
    parser.add_argument("--replay", default=os.environ.get("PLTRS_FUZZ_REPLAY"),
                        help="run the program of this seed only")
    parser.add_argument("--jobs", type=int, default=int(os.environ.get("PLTRS_FUZZ_JOBS", "4")))
    parser.add_argument("--no-reduce", action="store_true", help="keep bugs' programs whole")
    parser.add_argument("--compare", action="store_true",
                        help="run the programs under matplotlib too and write where their "
                             "exceptions differ to differences.md")
    arguments = parser.parse_args()
    bugs = runner.run(arguments.scale, arguments.replay, arguments.jobs,
                      not arguments.no_reduce, first=arguments.first,
                      compare_with_matplotlib=arguments.compare)
    print(runner.summary(bugs) if bugs else "no bug found")
    return 1 if runner.real(bugs) else 0


sys.exit(main())
