"""
Programs running the matplotlib-code references of pltrs (Python)
(``*_reference.py``: matplotlib's code nearly as it is, which pltrs runs
where it cannot make what a program asks itself): one program for each
function no other test runs, and one for each option a program chooses
of the plotting methods made of plotting methods (``errorbar``,
``violinplot``, ``stackplot``, ``pie_label`` and those of
``_composed_reference``) on an Axes class overriding the methods they are
made of. A program makes a figure and returns what the method returned;
`fingerprint` notes both, `reference_cases.json` holds matplotlib's.

    python reference_cases.py --record    (with matplotlib importable)

records them again, from a run of matplotlib 3.11.2.
"""

import datetime
import importlib
import json
import pathlib
import sys
import types

import numpy as np

GOLDEN = pathlib.Path(__file__).with_name("reference_cases.json")

#: The methods an Axes class of the program's overrides (each passing to
#: matplotlib's), which make the methods built of them run matplotlib's code.
ROUTED = ("plot", "fill", "bar", "barh", "fill_between", "fill_betweenx", "vlines", "hlines",
          "text", "annotate")


def modules(package):
    """The modules of *package* (``matplotlib`` or ``pltrs``) the
    programs use."""
    names = ("pyplot", "axes", "projections", "ticker", "dates")
    found = {name: importlib.import_module(f"{package}.{name}") for name in names}
    found["process_plot_var_args"] = importlib.import_module(
        f"{package}.axes._base")._process_plot_var_args
    return types.SimpleNamespace(**found)


def routed(base):
    """A class of *base* overriding `ROUTED`."""
    def passing(name):
        def method(self, *args, **kwargs):
            return getattr(super(cls, self), name)(*args, **kwargs)
        method.__name__ = name
        return method
    cls = type(f"Routed{base.__name__}", (base,), {name: passing(name) for name in ROUTED})
    return cls


def _axes(m, polar=False):
    fig = m.pyplot.figure(figsize=(4, 3))
    base = m.projections.PolarAxes if polar else m.axes.Axes
    return fig, fig.add_subplot(axes_class=routed(base))


X = np.linspace(0.0, 6.0, 64)
SIGNAL = np.sin(2 * X) + 0.5 * np.cos(7 * X)
OTHER = np.cos(3 * X) + 0.25 * np.sin(11 * X)
DATA = [np.linspace(-1, 1, 9) ** 3, np.linspace(0, 2, 11) ** 2]


def _spectrum(name, **kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, getattr(ax, name)(SIGNAL, Fs=10, **kwargs)
    return case


def _two_signals(name, **kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, getattr(ax, name)(SIGNAL, OTHER, NFFT=32, Fs=10, **kwargs)
    return case


def _hist(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, ax.hist(DATA, bins=4, **kwargs)
    return case


def _bxp(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        stats = [dict(med=1, q1=0.5, q3=1.5, whislo=0, whishi=2, fliers=[3, -1], mean=1.2,
                      cilo=0.8, cihi=1.2, label="a"),
                 dict(med=2, q1=1.5, q3=2.5, whislo=1, whishi=3, fliers=[], mean=2.1,
                      cilo=1.8, cihi=2.2, label="b")]
        return fig, ax.bxp(stats, **kwargs)
    return case


def _ecdf(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, ax.ecdf([3.0, 1.0, 2.0, 2.0, 5.0], **kwargs)
    return case


def _bar_label(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        bars = ax.bar([0, 1, 2], [3.0, -1.5, 2.25])
        return fig, ax.bar_label(bars, **kwargs)
    return case


def _violin(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, ax.violinplot(DATA, **kwargs)
    return case


def _stackplot(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        return fig, ax.stackplot([0, 1, 2, 3], [1, 2, 1, 3], [2, 1, 2, 1], **kwargs)
    return case


def _errorbar(polar=False, **kwargs):
    def case(m):
        fig, ax = _axes(m, polar)
        y = np.linspace(1, 2, 6)
        return fig, ax.errorbar(np.arange(6.0) / 2, y, **kwargs)
    return case


def _errorbar_dates(m):
    fig, ax = _axes(m)
    days = [datetime.datetime(2024, 1, day) for day in (1, 3, 5)]
    return fig, ax.errorbar(days, [1.0, 2.0, 1.5], xerr=datetime.timedelta(days=1), yerr=0.25,
                            capsize=3)


def _pie_label(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        pie = ax.pie([3, 2, 1])
        return fig, ax.pie_label(pie, **kwargs)
    return case


def _make_coordinates(m):
    fig, ax = _axes(m)
    cycle = m.process_plot_var_args(output="coordinates")
    return fig, cycle(ax, [0, 1, 2], [2, 0, 1])


def _logit_format_value(m):
    formatter = m.ticker.LogitFormatter()
    locs = np.array([0.01, 0.1, 0.3, 0.5, 0.9])
    return None, [formatter._format_value(x, locs) for x in (0.1, 0.3, 0.9)]


def _logit_one_minus(m):
    formatter = m.ticker.LogitFormatter(one_half="½")
    formatter.set_locs([0.01, 0.5, np.nan, 0.99])
    return None, [formatter(x) for x in (0.01, 0.5, 0.99)]


def _quiver(**kwargs):
    def case(m):
        fig, ax = _axes(m)
        u, v = np.meshgrid(np.linspace(-1, 1, 3), np.linspace(0.5, 1, 3))
        q = ax.quiver(u, v, angles="xy", scale_units="xy", **kwargs)
        fig.canvas.draw()
        return fig, q
    return case


def _pie_usetex_props(m):
    fig, ax = _axes(m)
    return fig, ax.pie([1, 2, 3], autopct="%1.0f%%", textprops={"usetex": False, "size": 9})


def _hexbin_marginals(m):
    fig, ax = _axes(m)
    hb = ax.hexbin(np.cos(X), np.sin(3 * X), gridsize=5, mincnt=0.5, marginals=True)
    hb.set_cmap("magma")
    return fig, [hb.hbar.get_cmap().name, hb.vbar.get_clim()]


CASES = {
    "psd": _spectrum("psd", NFFT=32),
    "psd-twosided-return_line": _spectrum("psd", NFFT=32, sides="twosided", detrend="mean",
                                          return_line=True),
    "csd": _two_signals("csd"),
    "csd-return_line": _two_signals("csd", return_line=True),
    "cohere": _two_signals("cohere"),
    "magnitude_spectrum": _spectrum("magnitude_spectrum"),
    "magnitude_spectrum-dB": _spectrum("magnitude_spectrum", scale="dB"),
    "angle_spectrum": _spectrum("angle_spectrum"),
    "phase_spectrum": _spectrum("phase_spectrum"),
    "hist-bar": _hist(),
    "hist-barstacked-weights-bottom": _hist(histtype="barstacked",
                                            weights=[np.ones(9), np.full(11, 0.5)], bottom=1),
    "hist-stepfilled-horizontal-cumulative-log": _hist(histtype="stepfilled",
                                                       orientation="horizontal",
                                                       cumulative=True, log=True),
    "hist-step-stacked-density": _hist(histtype="step", stacked=True, density=True),
    "bxp": _bxp(),
    "bxp-horizontal-means-patch-notch": _bxp(orientation="horizontal", showmeans=True,
                                             meanline=True, patch_artist=True,
                                             shownotches=True),
    "bxp-no-caps-box-fliers": _bxp(showcaps=False, showbox=False, showfliers=False),
    "ecdf": _ecdf(),
    "ecdf-complementary-horizontal-compressed": _ecdf(complementary=True,
                                                      orientation="horizontal", compress=True),
    "bar_label-edge": _bar_label(),
    "bar_label-center-format-padding": _bar_label(label_type="center", fmt="{:.1f}", padding=3),
    "bar_label-labels": _bar_label(labels=["a", "b", "c"]),
    "violinplot": _violin(),
    "violinplot-means-medians-quantiles": _violin(showmeans=True, showmedians=True,
                                                  quantiles=[[0.25, 0.75], [0.5]]),
    "violinplot-horizontal-low-side": _violin(orientation="horizontal", side="low",
                                              showextrema=False),
    "violinplot-colors": _violin(facecolor=["r", "g"], linecolor="k"),
    "stackplot-zero-labels-colors": _stackplot(labels=["a", "b"], colors=["r", "b"]),
    "stackplot-sym": _stackplot(baseline="sym"),
    "stackplot-wiggle": _stackplot(baseline="wiggle"),
    "stackplot-weighted_wiggle": _stackplot(baseline="weighted_wiggle"),
    "errorbar-limits": _errorbar(yerr=0.2, xerr=0.1, lolims=[True, False] * 3,
                                 uplims=[False, True] * 3, xlolims=True, capsize=4),
    "errorbar-asymmetric-styled": _errorbar(yerr=[[0.1] * 6, [0.3] * 6], fmt="o-", ecolor="r",
                                            elinewidth=2, capsize=2, capthick=3,
                                            barsabove=True),
    "errorbar-every-int": _errorbar(yerr=0.2, errorevery=2),
    "errorbar-every-tuple": _errorbar(yerr=0.2, xerr=0.1, errorevery=(1, 2)),
    "errorbar-every-slice": _errorbar(yerr=0.2, errorevery=slice(1, None, 3)),
    "errorbar-every-mask": _errorbar(yerr=0.2, errorevery=[True, False, False, True, True, False]),
    "errorbar-polar": _errorbar(polar=True, yerr=0.2, xerr=0.1, capsize=3),
    "errorbar-dates": _errorbar_dates,
    "pie_label-list-rotated": _pie_label(labels=["a", "b", "c"], rotate=True),
    "pie_label-format-distance": _pie_label(labels="{frac:.1%}", distance=0.6,
                                            alignment="center"),
    "plot_args-coordinates": _make_coordinates,
    "logit-format_value": _logit_format_value,
    "logit-one_minus": _logit_one_minus,
    "quiver-xy-array-width": _quiver(width=np.array([0.01])),
    "quiver-xy-unusable-scale": _quiver(scale="big"),
    "pie-autopct-textprops": _pie_usetex_props,
    "hexbin-marginals-changed": _hexbin_marginals,
}


def _value(value, depth=0):
    """*value* in a notation of its own: numbers rounded, arrays by their
    first values, artists by their type."""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, (int, float, np.integer, np.floating)):
        return round(float(value), 4)
    if isinstance(value, np.ndarray):
        flat = np.ravel(np.ma.filled(value.astype(float), np.nan)) if value.dtype.kind in "biuf" else []
        return ["array", list(value.shape), [_value(v) for v in flat[:6]]]
    if isinstance(value, (list, tuple)) and depth < 3:
        return [_value(item, depth + 1) for item in value[:8]]
    return type(value).__name__


def _artist(artist):
    """What a plotted artist shows."""
    kind = type(artist).__name__
    if hasattr(artist, "get_xydata"):
        return [kind, _value(artist.get_xydata()), _value(len(artist.get_xydata()))]
    if hasattr(artist, "get_patch_transform"):
        extents = artist.get_path().get_extents(artist.get_patch_transform())
        return [kind, _value(extents.bounds), _value(artist.get_facecolor())]
    if hasattr(artist, "get_offsets"):
        paths = artist.get_paths()
        return [kind, len(paths), _value(paths[0].vertices) if paths else None,
                _value(artist.get_offsets()), _value(artist.get_facecolor())]
    if hasattr(artist, "get_text"):
        return [kind, artist.get_text(), _value(artist.get_position()),
                _value(artist.get_rotation())]
    return kind


def fingerprint(fig, result):
    """What a program made: its Axes' limits and plotted artists, and the
    value it returned."""
    noted = {"result": _value(result if not hasattr(result, "__iter__") or isinstance(result, str)
                              else list(result))}
    if fig is not None:
        fig.canvas.draw()
        noted["axes"] = [
            {"xlim": _value(ax.get_xlim()), "ylim": _value(ax.get_ylim()),
             "artists": [_artist(a) for a in (*ax.lines, *ax.patches, *ax.collections, *ax.texts)]}
            for ax in fig.axes]
    return noted


def run(name, package):
    """The fingerprint of the program *name* under *package*, or its
    exception."""
    m = modules(package)
    try:
        fig, result = CASES[name](m)
        return fingerprint(fig, result)
    except Exception as error:  # noqa: BLE001 - the program's failure is what is compared
        return {"raises": type(error).__name__, "message": str(error)[:120]}
    finally:
        m.pyplot.close("all")


if __name__ == "__main__":
    if sys.argv[1:] != ["--record"]:
        sys.exit("usage: python reference_cases.py --record")
    import matplotlib
    matplotlib.use("agg")
    matplotlib.rcdefaults()
    recorded = {name: run(name, "matplotlib") for name in CASES}
    GOLDEN.write_text(json.dumps(recorded, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
