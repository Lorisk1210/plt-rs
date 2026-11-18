"""
A fuzzer for pltrs (Python): seeded random programs, looking for what no
program may meet, whatever it does: a Rust panic, a crash, a hang or
runaway memory, a `SystemError`, or pltrs refusing a change to a figure
it is laying out. Other Python exceptions are fine, as matplotlib raises
them too; ``--compare`` runs the programs under matplotlib as well and
lists the lines raising otherwise (`compare`).

The programs are the fuzzer's own (`actions`: random calls of the public
API with valid, borderline and absurd arguments, property changes between
draws, saves in every format, pickles, events, callbacks changing the
figure while it is drawn; and, for odd seeds, `paths`: subclasses
overriding what libraries override, private names, callbacks entering
their figure, assigned attributes) and the differential tester's
(`corpus`, every family in turn, mutated). Workers run them (`worker`), the runner (`runner`) reduces each
bug to a minimal program in ``target/python-fuzz/failures``.

    PYTHONPATH=python:python/tests .venv-pltrs/bin/python -m fuzz [--scale N]

or through pytest (``python/tests/fuzz/test_fuzz.py``, part of ``cargo
xtask python test``). ``PLTRS_FUZZ_SCALE=50`` runs 50 times as many
programs, ``PLTRS_FUZZ_REPLAY=<seed>`` reruns one, as with the Rust fuzz
tests; ``PLTRS_FUZZ_FIRST=<seed>`` starts from another seed (programs
not run yet), ``PLTRS_FUZZ_JOBS`` sets how many workers run at once,
``PLTRS_FUZZ_DIR`` the directory of the corpus and the failures (by
default ``target/python-fuzz``).
The seeds that met a bug once run first in every run
(``runner.FAILED_SEEDS``), and on Linux and macOS the workers run with a
1 MiB stack, so that a recursion through pltrs (Rust)'s drawing meets the
end of the stack in a development build as in an optimised one.
"""
