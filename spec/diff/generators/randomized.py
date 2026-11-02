"""Random sequences of plotting calls, setters and getters.

A SplitMix64 stream (``spec_support.rng``'s) chooses, for each program,
a few plotting calls from ``plotting.VARIANTS`` on one or two axes, then
setters with samples from ``artists.VALUES`` on the artists they made and
on the axes, each followed by its getter, then the getters of what was
touched, before and after a draw. The seed of a program is its number, so
a program reads the same from one run to the next.
"""

from __future__ import annotations

import pathlib
import sys
from typing import Any, Iterator

from . import artists, common, plotting

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2] / "reference"))
from spec_support.data import SpecRng  # noqa: E402

#: How many programs, and how many plotting calls each.
PROGRAMS = 60
CALLS = 4
SETS = 8

#: Methods whose results are artists with setters (or lists of them).
ARTIST_METHODS = ["plot", "scatter", "bar", "fill_between", "imshow", "pcolormesh", "contourf", "text",
                  "annotate", "axhline", "axvspan", "stairs", "hexbin", "quiver", "step", "errorbar",
                  "hist", "stem", "eventplot", "pie", "legend"]


def _choice(rng: SpecRng, items: list[Any]) -> Any:
    return items[(rng.next_u64() >> 11) * len(items) >> 53]


def _first_artist(value: Any) -> str | None:
    """The expression of the first artist of a plotting call's result."""
    from matplotlib.artist import Artist

    if isinstance(value, Artist):
        return "result"
    if isinstance(value, (list, tuple)):
        for index, item in enumerate(value):
            inner = _first_artist(item)
            if inner is not None:
                return inner.replace("result", f"result[{index}]", 1)
    return None


def _program(seed: int) -> list[str]:
    rng = SpecRng(seed)
    statements = list(common.HEADER + common.DATA)
    axes = 1 + (rng.next_u64() & 1)
    statements.append(f"fig, axs = plt.subplots(1, {axes}, squeeze=False)")
    for call in range(CALLS):
        method = _choice(rng, ARTIST_METHODS)
        arguments = _choice(rng, plotting.VARIANTS[method])
        statements.append(f"ax = axs[0, {call % axes}]")
        statements.append(f"result = ax.{method}({arguments})")
        try:
            scope = common.namespace(statements)
        except Exception:  # noqa: BLE001 - a failing call is compared as it is
            continue
        target = _first_artist(scope.get("result"))
        if target is None:
            continue
        statements.append(f"obj = {target}")
        obj = eval(target, scope)  # noqa: S307 - the generator's own expression
        candidates = [p for p in common.setters(obj) if artists.VALUES.get(p)]
        for _ in range(SETS // CALLS):
            if not candidates:
                break
            prop = _choice(rng, candidates)
            statements.append(f"obj.set_{prop}({_choice(rng, artists.VALUES[prop])})")
            if callable(getattr(obj, f"get_{prop}", None)):
                statements.append(f"obj.get_{prop}()")
        statements += [f"obj.{name}()" for name in common.getters(obj)]
    for prop in ("xlim", "ylim", "xscale", "title", "aspect", "facecolor"):
        if rng.next_f64() < 0.3:
            statements.append(f"ax.set_{prop}({_choice(rng, artists.VALUES[prop])})")
    statements += plotting.AFTER + ["fig.canvas.draw()"] + plotting.AFTER
    return statements


def programs() -> Iterator[tuple[str, list[str]]]:
    """The seeded random programs."""
    for seed in range(PROGRAMS):
        yield f"seed_{seed:03d}", _program(seed)
