"""Tick placement and labelling: every locator and formatter of the gallery,
minor ticks, log/symlog/logit/asinh/function scales, offsets and scientific
notation, tick parameters and rotated or multi-line tick labels."""

import numpy as np
from matplotlib import ticker

from spec_support import case, rng


def _strip(ax, title):
    """The gallery's one-spine strip: only the x axis carries ticks."""
    ax.yaxis.set_major_locator(ticker.NullLocator())
    ax.spines[["left", "right", "top"]].set_visible(False)
    ax.xaxis.set_ticks_position("bottom")
    ax.tick_params(which="major", width=1.00, length=5)
    ax.tick_params(which="minor", width=0.75, length=2.5)
    ax.set_xlim(0, 5)
    ax.set_ylim(0, 1)
    ax.patch.set_visible(False)
    ax.text(0.0, 0.2, title, transform=ax.transAxes, fontsize=12, color="tab:blue")


@case("ticks/locators_gallery", figsize=(8, 6))
def _(fig):
    axs = fig.subplots(8, 1)
    _strip(axs[0], "NullLocator()")
    axs[0].xaxis.set_major_locator(ticker.NullLocator())
    axs[0].xaxis.set_minor_locator(ticker.NullLocator())
    _strip(axs[1], "MultipleLocator(0.5, offset=0.2)")
    axs[1].xaxis.set_major_locator(ticker.MultipleLocator(0.5, offset=0.2))
    axs[1].xaxis.set_minor_locator(ticker.MultipleLocator(0.1))
    _strip(axs[2], "FixedLocator([0, 1, 5])")
    axs[2].xaxis.set_major_locator(ticker.FixedLocator([0, 1, 5]))
    axs[2].xaxis.set_minor_locator(ticker.FixedLocator(np.linspace(0.2, 0.8, 4)))
    _strip(axs[3], "LinearLocator(numticks=3)")
    axs[3].xaxis.set_major_locator(ticker.LinearLocator(3))
    axs[3].xaxis.set_minor_locator(ticker.LinearLocator(31))
    _strip(axs[4], "IndexLocator(base=0.5, offset=0.25)")
    axs[4].plot([0] * 5, color="white")
    axs[4].xaxis.set_major_locator(ticker.IndexLocator(base=0.5, offset=0.25))
    _strip(axs[5], "AutoLocator()")
    axs[5].xaxis.set_major_locator(ticker.AutoLocator())
    axs[5].xaxis.set_minor_locator(ticker.AutoMinorLocator())
    _strip(axs[6], "MaxNLocator(n=4)")
    axs[6].xaxis.set_major_locator(ticker.MaxNLocator(4))
    axs[6].xaxis.set_minor_locator(ticker.MaxNLocator(40))
    _strip(axs[7], "LogLocator(base=10, numticks=15)")
    axs[7].set_xlim(10**3, 10**10)
    axs[7].set_xscale("log")
    axs[7].xaxis.set_major_locator(ticker.LogLocator(base=10, numticks=15))
    fig.tight_layout()


@case("ticks/formatters_gallery", figsize=(8, 7))
def _(fig):
    axs = fig.subplots(9, 1)
    for ax in axs:
        _strip(ax, "")
        ax.xaxis.set_major_locator(ticker.MultipleLocator(1.00))
        ax.xaxis.set_minor_locator(ticker.MultipleLocator(0.25))
        ax.tick_params(which="minor", labelsize=10)
    fig.suptitle("Tick formatters")
    axs[0].xaxis.set_major_formatter("{x} km")
    axs[1].xaxis.set_major_formatter(lambda x, pos: str(x - 5))
    axs[2].xaxis.set_major_formatter(ticker.NullFormatter())
    axs[3].xaxis.set_major_formatter(ticker.StrMethodFormatter("{x:.3f}"))
    axs[4].xaxis.set_major_formatter(ticker.FuncFormatter(lambda x, pos: f"[{x:.2f}]"))
    axs[5].xaxis.set_major_locator(ticker.FixedLocator([0, 1, 2, 3, 4, 5]))
    axs[5].xaxis.set_major_formatter(ticker.FixedFormatter(["A", "B", "C", "D", "E", "F"]))
    axs[6].xaxis.set_major_formatter(ticker.ScalarFormatter(useMathText=True))
    axs[7].xaxis.set_major_formatter(ticker.FormatStrFormatter("#%d"))
    axs[8].xaxis.set_major_formatter(ticker.PercentFormatter(xmax=5))
    fig.tight_layout()


@case("ticks/maxn_integer_symmetric_prune", figsize=(7, 3))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.plot([0.2, 3.7], [0.1, 2.6], "o-")
    ax1.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax1.yaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax2.plot([-2.0, 5.0], [-0.3, 1.2])
    ax2.xaxis.set_major_locator(ticker.MaxNLocator(nbins=4, symmetric=True))
    ax2.yaxis.set_major_locator(ticker.MaxNLocator(nbins=5, prune="both"))
    ax3.plot([0, 100], [0, 1e-3])
    ax3.xaxis.set_major_locator(ticker.MaxNLocator(nbins=3, steps=[1, 3, 10], prune="lower"))
    ax3.yaxis.set_major_locator(ticker.MaxNLocator(nbins=6, min_n_ticks=4, prune="upper"))
    fig.tight_layout()


@case("ticks/minor_locators_and_params", figsize=(6, 4.5))
def _(fig):
    axs = fig.subplots(2, 2)
    x = np.linspace(0, 7, 50)
    for ax in axs.flat:
        ax.plot(x, np.cos(x) * 3)
    axs[0, 0].xaxis.set_minor_locator(ticker.AutoMinorLocator(4))
    axs[0, 0].yaxis.set_minor_locator(ticker.AutoMinorLocator())
    axs[0, 1].xaxis.set_major_locator(ticker.MultipleLocator(2))
    axs[0, 1].xaxis.set_minor_locator(ticker.MultipleLocator(0.5))
    axs[0, 1].tick_params(which="minor", length=6, color="tab:red", width=1.5)
    axs[1, 0].minorticks_on()
    axs[1, 0].tick_params(which="minor", direction="in", top=True, right=True, length=3)
    axs[1, 0].tick_params(which="major", direction="inout", length=8)
    axs[1, 1].xaxis.set_major_locator(ticker.MultipleLocator(1))
    axs[1, 1].xaxis.set_minor_locator(ticker.MultipleLocator(0.5))
    axs[1, 1].xaxis.remove_overlapping_locs = False
    axs[1, 1].tick_params(which="minor", length=10, color="tab:green")
    axs[1, 1].yaxis.set_minor_locator(ticker.FixedLocator([-2.5, -0.5, 0.5, 2.5]))
    axs[1, 1].yaxis.set_minor_formatter(ticker.FormatStrFormatter("%.1f"))
    axs[1, 1].tick_params(axis="y", which="minor", labelsize=7, labelcolor="tab:green")
    fig.tight_layout()


@case("ticks/minor_ndivs_rc", figsize=(5, 3.5), rc={"xtick.minor.visible": True, "ytick.minor.visible": True, "xtick.minor.ndivs": 2, "ytick.minor.ndivs": 5})
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 9], [0, 0.9])


@case("ticks/eng_formatter", figsize=(7, 7))
def _(fig):
    g = rng(19680801)
    xs = np.logspace(1, 9, 100)
    ys = (0.8 + 0.4 * g.uniform(100)) * np.log10(xs) ** 2
    ax0, ax1, ax2 = fig.subplots(3, 1)
    for ax in (ax0, ax1):
        ax.set_xscale("log")
        ax.plot(xs, ys)
    ax0.set_title("Full unit ticklabels, w/ default precision & space separator")
    ax0.xaxis.set_major_formatter(ticker.EngFormatter(unit="Hz"))
    ax0.set_xlabel("Frequency")
    ax1.set_title("SI-prefix only ticklabels, 1-digit precision & thin space separator")
    ax1.xaxis.set_major_formatter(ticker.EngFormatter(places=1, sep="\N{THIN SPACE}"))
    ax1.set_xlabel("Frequency [Hz]")
    ax2.set_title("Offset and SI prefix")
    t = np.linspace(0, 1e-3, 50)
    ax2.plot(t, 3e9 + 2e5 * np.sin(t * 6e3))
    ax2.xaxis.set_major_formatter(ticker.EngFormatter(unit="s"))
    ax2.yaxis.set_major_formatter(ticker.EngFormatter(unit="Hz", useOffset=True))
    fig.tight_layout()


@case("ticks/log_minor_labels", figsize=(7, 3.5))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.semilogy([0, 1], [2, 9])
    ax2.semilogy([0, 1], [1, 60])
    ax3.semilogy([0, 1], [1, 1e3])
    ax3.yaxis.set_minor_formatter(ticker.LogFormatter(labelOnlyBase=False, minor_thresholds=(4, 0.5)))
    fig.tight_layout()


@case("ticks/log_formatter_variants", figsize=(6, 5))
def _(fig):
    axs = fig.subplots(4, 1)
    for ax in axs:
        ax.set_xscale("log")
        ax.set_xlim(1e-2, 1e4)
        ax.yaxis.set_major_locator(ticker.NullLocator())
    axs[0].xaxis.set_major_formatter(ticker.LogFormatterExponent())
    axs[1].xaxis.set_major_formatter(ticker.LogFormatterMathtext())
    axs[2].xaxis.set_major_formatter(ticker.LogFormatter(labelOnlyBase=True))
    axs[3].set_xscale("log", base=2)
    axs[3].set_xlim(1 / 8, 512)
    axs[3].xaxis.set_major_formatter(ticker.LogFormatterSciNotation(base=2))
    fig.tight_layout()


@case("ticks/log_wide_range_decimation", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.logspace(-12, 12, 25)
    ax.loglog(x, x**0.5, "o-")
    ax.grid(which="both", alpha=0.4)


@case("ticks/log_locator_subs", figsize=(6, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = np.logspace(0, 3, 30)
    ax1.loglog(x, x)
    ax1.yaxis.set_major_locator(ticker.LogLocator(base=10, subs=(1.0, 2.0, 5.0)))
    ax1.yaxis.set_major_formatter(ticker.LogFormatter(labelOnlyBase=False))
    ax2.semilogx(x, np.log(x))
    ax2.xaxis.set_major_locator(ticker.LogLocator(numticks=3))
    ax2.xaxis.set_minor_locator(ticker.LogLocator(subs="auto"))
    fig.tight_layout()


@case("ticks/symlog_subs_minor", figsize=(6, 3.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = np.linspace(-1000, 1000, 401)
    ax1.plot(x, x)
    ax1.set_yscale("symlog", linthresh=1, subs=[2, 5])
    ax1.grid(which="minor", alpha=0.3)
    ax2.plot(x, x / 100)
    ax2.set_xscale("symlog", linthresh=10, base=2, linscale=0.5)
    ax2.set_yscale("symlog", linthresh=0.1)
    fig.tight_layout()


def _cdfs(x):
    logistic = 1 / (1 + np.exp(-x))
    laplace = np.where(x < 0, 1 / 2 * np.exp(x), 1 - 1 / 2 * np.exp(-x))
    cauchy = np.arctan(x) / np.pi + 1 / 2
    return logistic, laplace, cauchy


@case("ticks/logit_demo", figsize=(6.4, 8.5))
def _(fig):
    x = np.linspace(-10, 10, 1000)
    curves = _cdfs(x)
    axs = fig.subplots(3, 2)
    for ax in axs.flat:
        for y, label in zip(curves, ("logistic", "laplacian", "cauchy")):
            ax.plot(x, y, label=label)
        ax.legend()
        ax.grid()
    axs[0, 0].set(title="logit scale")
    axs[0, 0].set_yscale("logit")
    axs[0, 0].set_ylim(1e-5, 1 - 1e-5)
    axs[0, 1].set(title="logit scale")
    axs[0, 1].set_yscale("logit")
    axs[0, 1].set_xlim(0, 10)
    axs[0, 1].set_ylim(0.8, 1 - 5e-3)
    axs[1, 0].set(title="logit scale")
    axs[1, 0].set_yscale("logit", one_half="1/2", use_overline=True)
    axs[1, 0].set_ylim(1e-5, 1 - 1e-5)
    axs[1, 1].set(title="logit scale")
    axs[1, 1].set_yscale("logit", one_half="1/2", use_overline=True)
    axs[1, 1].set_xlim(0, 10)
    axs[1, 1].set_ylim(0.8, 1 - 5e-3)
    axs[2, 0].set(title="linear scale")
    axs[2, 0].set_ylim(0, 1)
    axs[2, 1].set(title="linear scale")
    axs[2, 1].set_xlim(0, 10)
    axs[2, 1].set_ylim(0.8, 1)
    fig.tight_layout()


@case("ticks/asinh_demo", figsize=(7, 3.5))
def _(fig):
    x = np.linspace(-3, 600, 500)
    ax0, ax1, ax2 = fig.subplots(1, 3, sharex=True)
    for ax in (ax0, ax1, ax2):
        ax.plot(x, x)
        ax.grid()
    ax0.set_title("symlog")
    ax0.set_yscale("symlog")
    ax1.set_title("asinh")
    ax1.set_yscale("asinh")
    ax2.set_title("asinh, base 2")
    ax2.set_yscale("asinh", linear_width=0.5, base=2)
    fig.tight_layout()


def _mercator(a):
    a = np.deg2rad(a)
    return np.rad2deg(np.log(np.abs(np.tan(a) + 1.0 / np.cos(a))))


def _inverse_mercator(a):
    a = np.deg2rad(a)
    return np.rad2deg(np.arctan(np.sinh(a)))


@case("ticks/function_scale_mercator", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(0, 170.0, 0.1)
    ax.plot(t, t / 2.0, "-", lw=2)
    ax.set_yscale("function", functions=(_mercator, _inverse_mercator))
    ax.set_title("function: Mercator")
    ax.grid(True)
    ax.set_xlim([0, 180])
    ax.yaxis.set_minor_formatter(ticker.NullFormatter())
    ax.yaxis.set_major_locator(ticker.FixedLocator(np.arange(0, 90, 10)))


@case("ticks/offset_threshold_and_fixed_offset", figsize=(7, 3), rc={"axes.formatter.offset_threshold": 2})
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    t = np.linspace(0, 1, 20)
    ax1.plot(t, 1000.1 + 0.8 * t)
    ax2.plot(t, 1000.1 + 0.8 * t)
    ax2.ticklabel_format(axis="y", useOffset=1000.0)
    ax3.plot(-1e5 - 3 * t, -2e-3 * t)
    fig.tight_layout()


@case("ticks/mathtext_offset_and_scale", figsize=(6, 3.5), rc={"axes.formatter.use_mathtext": True, "axes.formatter.limits": (-3, 3)})
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    t = np.linspace(0, 1, 20)
    ax1.plot(t * 5e-5, t * 4e4)
    ax2.plot(t, 1e7 + 2e3 * t)
    fig.tight_layout()


@case("ticks/rotated_multiline_labels", figsize=(6, 4.5))
def _(fig):
    ax = fig.subplots()
    ax.plot(np.arange(5), [3, 1, 4, 1, 5])
    ax.set_xticks(
        np.arange(5),
        ["one\nline", "two\nlines\nhere", "three", "four\n4", "five"],
        rotation=30,
        ha="right",
        rotation_mode="anchor",
    )
    ax.set_yticks([1, 3, 5], ["low\nvalue", "mid", "high\nvalue"], rotation=15, multialignment="center")
    ax.set_xlabel("category")
    fig.subplots_adjust(bottom=0.25, left=0.2)


@case("ticks/rotation_mode_xtick_ytick", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 10], [0, 1000])
    ax.tick_params(axis="x", labelrotation=40, labelrotation_mode="xtick")
    ax.tick_params(axis="y", labelrotation=-25, labelrotation_mode="ytick")
    ax.tick_params(top=True, labeltop=True, right=True, labelright=True)
    fig.subplots_adjust(left=0.15, right=0.85, bottom=0.18, top=0.82)


@case("ticks/tick_params_grid_and_sides", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(np.linspace(0, 4, 30), np.linspace(0, 4, 30) ** 2)
    ax.grid(True)
    ax.tick_params(grid_color="tab:orange", grid_alpha=0.6, grid_linewidth=2, grid_linestyle=":")
    ax.tick_params(axis="y", which="both", direction="inout", length=8, left=False, labelleft=False, right=True, labelright=True)
    ax.tick_params(axis="x", pad=10, labelcolor="tab:purple", labelsize="large", width=2)


@case("ticks/label_positions_top_right", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(np.linspace(0, 1, 20), 2e6 * np.linspace(0, 1, 20) ** 2)
    ax.xaxis.tick_top()
    ax.xaxis.set_label_position("top")
    ax.yaxis.tick_right()
    ax.yaxis.set_label_position("right")
    ax.set_xlabel("x on top")
    ax.set_ylabel("y on the right")
    ax.set_title("title above the top tick labels")


@case("ticks/set_ticks_labels_minor_kwargs", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 10], [0, 1])
    ax.set_xticks([0, 5, 10], ["start", "middle", "end"], color="tab:blue", fontsize=12)
    ax.set_xticks([2.5, 7.5], ["a", "b"], minor=True, color="tab:red", fontsize=8)
    ax.set_yticks([0.25, 0.75], minor=True)
    ax.tick_params(axis="x", which="minor", pad=12)
