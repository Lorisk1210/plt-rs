"""The values the ``hostile`` generator gives a parameter instead of its
representative one: what users get wrong.

``variants(value, text, name, choices)`` takes the representative value
(``value``, what ``text`` evaluates to in the subject's program) and
returns, by its kind, the source of other values:

* numbers: negative, zero, NaN, inf, a numpy scalar, a string, None;
* booleans: None, an int, a string, numpy's boolean;
* strings: each choice of matplotlib's checks (``choices``), a choice
  that is none of them, the value in another case, empty, a number;
* points and short sequences: empty, one element, NaN in it, the other
  sequence type, a numpy array, one element too many, a string;
* data (numpy arrays): empty, one element, NaN and inf in it, masked, of
  ints, of booleans, as a list, as a pandas Series or DataFrame, as
  dates, one element short, of another dimension;
* objects, callables, dicts, dates: None, a number, a string, the
  wrong container.

Numbers that size what is drawn (``dpi``, a figure's size, counts) are
not given inf, so that a program draws in bounded memory and time.
"""

from __future__ import annotations

import datetime
import math
from typing import Any

import numpy as np

from .common import literal

#: Parameters never given inf (or huge values): they size pixels, counts
#: or waits, and a run would allocate or wait without bound.
BOUNDED = {
    "dpi", "figsize", "w", "h", "width", "height", "size_inches", "figwidth", "figheight", "NFFT", "pad_to",
    "noverlap", "num", "n", "N", "lutsize", "numpoints", "scatterpoints", "numVertices", "numvertices",
    "numsides", "resolution", "nbins", "numticks", "bins", "frames", "fps", "interval", "timeout", "maxn",
    "samples", "steps", "nth_coord", "levels", "density", "zoom", "scale", "mutation_scale", "fontsize", "size",
    "markersize", "ms", "linewidth", "lw", "pad", "nrows", "ncols", "rows", "cols", "ntimes", "max_n_ticks",
    "minor_thresholds", "subs", "stride", "rstride", "cstride", "rcount", "ccount", "radius", "length",
    "bootstrap", "nx", "ny", "lut", "dash_offset", "bitrate", "repeat_delay", "save_count", "cache_frame_data",
    "pause", "delay", "seconds", "fontsize_points", "ptsize", "hinting_factor", "points", "nsamples",
    "elinewidth", "capthick", "capsize", "linewidths", "markeredgewidth", "mew", "headwidth", "headlength",
    "headaxislength", "s", "sizes", "edgewidth",
}
#: Choices given per parameter, at most: longer lists (the colormaps'
#: names) are names rather than choices.
MAX_CHOICES = 40
#: Parameters naming a file: never given an int, which Python opens as a
#: file descriptor of the harness's.
FILES = {"fname", "filename", "filename_or_obj", "fp", "outfile", "file", "fh", "path", "dvi_path", "dirname",
         "fobj", "output", "tmpdir", "cache_dir"}
#: The largest data written out as a literal; larger data is changed by expressions.
MAX_LITERAL = 64


def _number(value: float, name: str, integral: bool) -> list[str]:
    out = ["-1" if integral else "-1.5", "0" if integral else "0.0", "float('nan')"]
    if name not in BOUNDED:
        out.append("float('inf')")
    if integral:
        out += [f"np.int64({int(value)})", f"{int(value)}.5", repr(str(int(value)))]
    else:
        out += [f"np.float32({value!r})" if math.isfinite(value) else "np.float32(0.5)", repr(str(value))]
    return out + ["None"]


def _string(value: str, choices: list[str]) -> list[str]:
    out = [c for c in choice_values(choices)[:-1] if c != repr(value)]
    swapped = value.swapcase() if value else "X"
    out += ["'nope'", repr(swapped)] if choices else ["''", repr(swapped)]
    return out + ["None", "3"]


def _sequence(value: list | tuple) -> list[str]:
    """A point or a short sequence: its edges and its other types."""
    empty, single = ("()", f"({literal(value[0])},)") if isinstance(value, tuple) else ("[]", f"[{literal(value[0])}]")
    out = [empty, single]
    if all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in value):
        with_nan = list(value)
        with_nan[0] = float("nan")
        other = list(value) if isinstance(value, tuple) else tuple(value)
        out += [literal(type(value)(with_nan)), literal(other), f"np.array({literal(list(value))})",
                literal(type(value)([*value, value[-1]])), "'ab'"]
    elif all(isinstance(v, str) for v in value):
        out += [literal(type(value)(range(len(value))))]
    return [o for o in out if o is not None] + ["None"]


def _dates(count: int) -> str:
    return f"np.datetime64('2024-01-01') + np.arange({count})"


def _array(value: np.ndarray, text: str, name: str) -> list[str]:
    """Data: its edges, its other dtypes and containers, and its shape
    changed."""
    shown = literal(value) if value.size <= MAX_LITERAL else None
    if shown is None or value.ndim == 0:
        written = f"np.asarray({text})"
        out = [f"{written}[:0]", f"{written}[:1]", f"{written} * np.nan", f"np.ma.masked_less({written}, "
               f"np.median({written}))", f"{written}[:-1]", f"{written}.tolist()"]
        return out + ["None", "'data'"]
    floats = value.astype(float)
    empty = f"np.empty({(0,) + value.shape[1:]!r})"
    single = literal(value[:1])
    with_nan = floats.copy().ravel()
    with_nan[0] = np.nan
    with_nan = with_nan.reshape(value.shape)
    with_inf = floats.copy().ravel()
    with_inf[-1] = np.inf
    with_inf = with_inf.reshape(value.shape)
    out = [empty, single, literal(with_nan), f"np.ma.masked_invalid({literal(with_nan)})"]
    if name not in BOUNDED:
        out.append(literal(with_inf))
    if value.dtype.kind == "f" and np.isfinite(value).all():
        out.append(literal(np.round(value).astype(np.int64)))
    out += [literal((floats > floats.mean()).astype(bool)), literal(value.tolist())]
    if value.ndim == 1:
        out += [f"pd.Series({literal(value.tolist())})", _dates(len(value)), literal(value[:-1]) if len(value) > 1
                else None, literal(value.reshape(1, -1))]
    elif value.ndim == 2:
        out += [f"pd.DataFrame({literal(value.tolist())})", literal(value[:-1]) if len(value) > 1 else None,
                literal(value.ravel())]
    return [o for o in out if o is not None] + ["None", "'data'"]


def _masked(value: np.ma.MaskedArray, text: str) -> list[str]:
    return [f"np.ma.masked_all({value.shape!r})", f"np.ma.getdata({text})", f"np.ma.masked_array({text}, True)",
            "None"]


def choice_values(choices: list[str]) -> list[str]:
    """The source of each choice (of lists of at most ``MAX_CHOICES``) and
    of one that is none of them."""
    if not choices or len(choices) > MAX_CHOICES:
        return []
    return [repr(c) for c in choices] + ["'nope'"]


def variants(value: Any, text: str, name: str, choices: list[str]) -> list[str]:
    """The source of the values given ``name`` instead of ``text`` (which
    is ``value``), the strings among them first."""
    if name in FILES:
        return ["None", "''", "io.StringIO()", "pathlib.Path('missing-directory') / 'file'"]
    if isinstance(value, (bool, np.bool_)):
        return ["None", "1", "'False'", "np.True_"]
    if isinstance(value, (int, np.integer)):
        return _number(float(value), name, integral=True)
    if isinstance(value, (float, np.floating)):
        return _number(float(value), name, integral=False)
    if isinstance(value, str):
        return _string(value, choices)
    if isinstance(value, np.ma.MaskedArray):
        return _masked(value, text)
    if isinstance(value, np.ndarray) and value.dtype.kind in "biuf":
        return _array(value, text, name)
    if isinstance(value, (list, tuple)) and value and len(value) <= MAX_LITERAL:
        return _sequence(value)
    if isinstance(value, (list, tuple)):
        return ["None", "'x'", "[None]"]
    if isinstance(value, dict):
        return ["{}", "{'nonexistent_key': 1}", "[]", "None"]
    if isinstance(value, (datetime.datetime, datetime.date, np.datetime64)):
        return ["None", "0", "np.datetime64('NaT')", "'2024-03-01'", "datetime.date(2024, 3, 1)"]
    if value is None:
        return choice_values(choices) or ["0", "'x'", "[]"]
    return ["None", "3", "'x'"]
