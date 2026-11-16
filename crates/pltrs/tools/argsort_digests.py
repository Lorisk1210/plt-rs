#!/usr/bin/env python3
"""Digests of numpy's ``argsort`` at each processor level, for the tests of
``pltrs::numeric::argsort``.

numpy's unstable ``argsort`` leaves equal values in an order that depends on
the code its wheel runs on the processor: its own introsort without AVX2
(``v2``), x86-simd-sort's AVX2 code (``v3``) or its AVX-512 code (``v4``).
pltrs replays all three as numpy 2.3 to 2.5 sort (earlier releases order
equal values otherwise in arrays of more than 64 values), and
``crates/pltrs/src/numeric/argsort/digests.txt`` holds what numpy 2.5.4
returns (``digests-windows.txt`` what its Windows wheel returns, whose
x86-simd-sort falls back to MSVC's ``std::sort``; that wheel has no AVX-512
code, so its ``v4`` column is ``-``): one line per family of seeded arrays
(ties, NaN, infinities, signed zeros, sorted, reversed, constant, stalled
partitions) and size, with a digest of the returned indices per level. A
size of ``0-600`` stands for every length from 0 to 600, chained into one
digest.

    argsort_digests.py            the digests of the numpy level in force
                                  (of any numpy: the way to compare two)
    argsort_digests.py --write    rewrite digests.txt, running numpy at each
                                  level this processor has (an AVX-512
                                  machine records all three; on Windows
                                  digests-windows.txt, from an AVX2 one)
    argsort_digests.py --check    compare the level in force with
                                  digests.txt: the proof on a processor of
                                  that level, with numpy alone
    argsort_digests.py --disabled LEVEL
                                  what NPY_DISABLE_CPU_FEATURES must hold
                                  for this numpy to run at LEVEL
    argsort_digests.py --probe    the digests, at each level, of the short
                                  arrays pltrs asks numpy to sort to tell
                                  how it sorts them (the constants of
                                  ``argsort/numpy.rs``'s tests)

The arrays come from SplitMix64 in integer arithmetic, which
``digests.rs`` and ``numpy.rs`` mirror, so they are the same bits
everywhere.
"""

import os
import pathlib
import subprocess
import sys

import numpy as np

#: The table of this system's numpy wheel.
DIGESTS = pathlib.Path(__file__).resolve().parents[1] / "src/numeric/argsort" / (
    "digests-windows.txt" if sys.platform == "win32" else "digests.txt")
LEVELS = ("v2", "v3", "v4")
#: What ``NPY_DISABLE_CPU_FEATURES`` takes away to run a numpy that
#: dispatches by groups (2.4 and later) at a level.
DISABLED_GROUPS = {
    "v2": "X86_V3 X86_V4 AVX512_ICL AVX512_SPR",
    "v3": "X86_V4 AVX512_ICL AVX512_SPR",
    "v4": "",
}
#: The lengths beyond the chained ones.
SIZES = (1000, 2600, 4096, 10_000, 65_537, 100_000, 1_000_000)
CHAINED = range(601)

GOLDEN = np.uint64(0x9E3779B97F4A7C15)
MIX1 = np.uint64(0xBF58476D1CE4E5B9)
MIX2 = np.uint64(0x94D049BB133111EB)


def _mix(z):
    """SplitMix64's output function."""
    z = (z ^ (z >> np.uint64(30))) * MIX1
    z = (z ^ (z >> np.uint64(27))) * MIX2
    return z ^ (z >> np.uint64(31))


def splitmix(seed, n):
    """The first ``n`` outputs of SplitMix64 from ``seed``."""
    steps = np.arange(1, n + 1, dtype=np.uint64)
    with np.errstate(over="ignore"):
        return _mix(np.uint64(seed) + steps * GOLDEN)


def _ties(levels):
    return lambda seed, n: (splitmix(seed, n) % np.uint64(levels)).astype(np.float64) / 4.0


def _uniform(seed, n):
    return (splitmix(seed, n) >> np.uint64(11)).astype(np.float64) / 2.0**53


def _nan(seed, n):
    values = _ties(4)(seed, n)
    values[::5] = np.nan
    return values


def _infinite(seed, n):
    values = _ties(4)(seed, n)
    values[::3] = np.inf
    values[1::7] = -np.inf
    return values


def _zeros(seed, n):
    return np.array([0.0, -0.0, 1.0, -1.0])[(splitmix(seed, n) % np.uint64(4)).astype(np.intp)]


def _sorted(seed, n):
    return (np.arange(n) // 3).astype(np.float64)


def _reversed(seed, n):
    return ((n - np.arange(n)) // 3).astype(np.float64)


def _constant(seed, n):
    return np.full(n, 2.5)


def _stalled(seed, n):
    i = np.arange(n)
    return np.where(i % 7 == 3, (i % 13).astype(np.float64), 0.0)


#: The families, in the order ``digests.rs`` numbers them for seeds.
FAMILIES = {
    "ties2": _ties(2),
    "ties5": _ties(5),
    "ties40": _ties(40),
    "ties1000": _ties(1000),
    "uniform": _uniform,
    "nan": _nan,
    "infinite": _infinite,
    "zeros": _zeros,
    "sorted": _sorted,
    "reversed": _reversed,
    "constant": _constant,
    "stalled": _stalled,
}


#: The seed of a probe array of ``n`` values is this plus ``n``.
PROBE_SEED = 20_261_114
#: The lengths of the arrays pltrs asks numpy to sort, of the family
#: ``ties5`` and, with NaN, of ``nan`` (``PROBES`` of ``argsort/numpy.rs``).
PROBES = (9, 20, 40, 64)


def digest(order):
    """A digest of the indices ``order``, sensitive to their positions."""
    with np.errstate(over="ignore"):
        positions = np.arange(order.size, dtype=np.uint64) * GOLDEN
        return int(_mix(order.astype(np.uint64) ^ positions).sum(dtype=np.uint64))


def family_digest(number, family, size):
    """The digest of ``np.argsort`` of the family's array of ``size``
    values, or of every length of ``CHAINED`` in turn."""
    def one(n):
        return digest(np.argsort(family(number * 10_007 + n, n)))

    if size != "0-600":
        return one(int(size))
    total = 0
    for n in CHAINED:
        total = (total * int(GOLDEN) + one(n)) % 2**64
    return total


def _umath():
    """numpy's extension module, which says what it dispatches to."""
    return (np._core if hasattr(np, "_core") else np.core)._multiarray_umath


def probe_digest(family):
    """The chained digest of ``np.argsort`` of the probe arrays of
    *family*."""
    total = 0
    for n in PROBES:
        order = np.argsort(FAMILIES[family](PROBE_SEED + n, n))
        total = (total * int(GOLDEN) + digest(order)) % 2**64
    return total


def level_in_force():
    """The level this numpy runs at: the highest whose copy of its loops
    the processor has and numpy holds (its baseline and dispatch lists),
    by its groups (numpy 2.4 and later) or by the features AVX2 and
    AVX-512 an earlier one dispatches by."""
    umath = _umath()
    return _highest({name for name, enabled in umath.__cpu_features__.items() if enabled})


def wheel_level():
    """The highest level this numpy holds a copy of its loops for,
    whatever the processor: ``v3`` for its Windows wheels, which hold no
    AVX-512 code."""
    return _highest(set(_umath().__cpu_features__))


def _highest(enabled):
    """The highest level among the groups (or, in a numpy without them,
    the features) *enabled* that numpy holds a copy of its loops for."""
    umath = _umath()
    runs = enabled & {*umath.__cpu_baseline__, *umath.__cpu_dispatch__}
    if "X86_V3" in umath.__cpu_features__:
        return "v4" if {"X86_V3", "X86_V4"} <= runs else "v3" if "X86_V3" in runs else "v2"
    return "v4" if "AVX512_SKX" in runs else "v3" if "AVX2" in runs else "v2"


def disabled(level):
    """What ``NPY_DISABLE_CPU_FEATURES`` takes away to run this numpy at
    *level*: groups, or for a numpy without them every AVX-512 feature it
    dispatches to and, for ``v2``, AVX2 and FMA as well."""
    dispatch = _umath().__cpu_dispatch__
    if "X86_V3" in dispatch:
        return DISABLED_GROUPS[level]
    wide = [name for name in dispatch if name.startswith("AVX512")]
    return " ".join({"v4": [], "v3": wide, "v2": ["FMA3", "AVX2", *wide]}[level])


def digests():
    """``{(family, size): digest}`` at the numpy level in force."""
    return {(name, str(size)): family_digest(number, family, size)
            for number, (name, family) in enumerate(FAMILIES.items())
            for size in ("0-600", *SIZES)}


def read():
    """digests.txt: ``{(family, size): {level: digest}}``."""
    table = {}
    for line in DIGESTS.read_text(encoding="utf-8").splitlines():
        if line and not line.startswith("#"):
            family, size, *values = line.split()
            table[family, size] = {level: None if value == "-" else int(value, 16)
                                   for level, value in zip(LEVELS, values)}
    return table


def at_each_level(*options):
    """What this tool prints with ``options`` under numpy run at each
    level its wheel holds code for, by level; needs a processor of the
    highest of them."""
    highest, wheel = level_in_force(), wheel_level()
    if highest != wheel:
        sys.exit(f"this numpy runs at {highest}: recording its levels needs a processor "
                 f"of {wheel}")
    printed = {}
    for level in LEVELS[:LEVELS.index(wheel) + 1]:
        env = dict(os.environ, NPY_DISABLE_CPU_FEATURES=disabled(level))
        out = subprocess.run([sys.executable, "-W", "ignore", __file__, *options], env=env,
                             check=True, capture_output=True, text=True).stdout.splitlines()
        if out[0] != f"# numpy {np.__version__} at {level}":
            sys.exit(f"numpy did not run at {level}: {out[0]}")
        printed[level] = out[1:]
    return printed


def write():
    """Record the table of this system's numpy wheel from numpy run at
    every level the wheel holds code for (``-`` for the others)."""
    columns = {level: [line.split() for line in lines]
               for level, lines in at_each_level().items()}
    lines = [
        f"# numpy {np.__version__}'s argsort of seeded arrays: family, size, and a digest of the",
        "# indices at v2, v3 and v4. Written by crates/pltrs/tools/argsort_digests.py --write.",
    ]
    unrecorded = ["-"] * (len(LEVELS) - len(columns))
    for rows in zip(*columns.values()):
        family, size = rows[0][:2]
        lines.append(f"{family} {size} " + " ".join([*(row[2] for row in rows), *unrecorded]))
    DIGESTS.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def probe():
    """Print the probe arrays' digests of this numpy at every level its
    wheel holds code for."""
    printed = at_each_level("--probe-here")
    print(f"numpy {np.__version__} for {sys.platform}: the probe arrays' digests at "
          + ", ".join(printed))
    for row, name in enumerate(("without NaN", "with NaN")):
        print(f"  {name}: " + ", ".join("0x" + lines[row] for lines in printed.values()))


def check():
    """Compare numpy here with its table at the level in force."""
    level, recorded, differing = level_in_force(), read(), []
    for key, value in digests().items():
        if recorded[key][level] != value:
            differing.append(key)
    print(f"numpy {np.__version__} at {level}: {len(recorded) - len(differing)} of "
          f"{len(recorded)} digests as recorded")
    for family, size in differing:
        print(f"  differs: {family} {size}")
    sys.exit(1 if differing else 0)


if __name__ == "__main__":
    if "--write" in sys.argv:
        write()
    elif "--check" in sys.argv:
        check()
    elif "--probe" in sys.argv:
        probe()
    elif "--probe-here" in sys.argv:
        print(f"# numpy {np.__version__} at {level_in_force()}")
        print(f"{probe_digest('ties5'):016x}\n{probe_digest('nan'):016x}")
    elif "--disabled" in sys.argv:
        print(disabled(sys.argv[sys.argv.index("--disabled") + 1]))
    else:
        print(f"# numpy {np.__version__} at {level_in_force()}")
        for (family, size), value in digests().items():
            print(f"{family} {size} {value:016x}")
