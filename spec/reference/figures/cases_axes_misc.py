"""Miscellaneous axes features: date axes, categorical axes, log/symlog
helpers, spectral plots, correlations, Sankey, inset indicators,
axes_grid1, axisartist, figure-level artists, property cycles and the
static appearance of widgets."""

from datetime import datetime, timedelta

import numpy as np
from matplotlib import dates as mdates
from matplotlib import lines, patches
from matplotlib import widgets
from matplotlib.sankey import Sankey
from mpl_toolkits import axisartist
from mpl_toolkits.axes_grid1 import AxesGrid, ImageGrid, make_axes_locatable

from spec_support import case, rng


def _wave(n):
    """n points spanning a smooth deterministic wave."""
    return np.sin(np.linspace(0.0, 3 * np.pi, n)) + np.linspace(0.0, 1.0, n)


def _dates(start, step, n):
    return [start + i * step for i in range(n)]


def _signal(n=1024, fs=100.0, seed=61):
    t = np.arange(n) / fs
    return np.sin(2 * np.pi * 10 * t) + 0.5 * np.sin(2 * np.pi * 23 * t) + 0.3 * rng(seed).normal(n)


def _field(n=20):
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(xx) * np.cos(yy)


# --- dates

@case("misc/dates_minutes", figsize=(6.4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2024, 3, 1, 12, 0), timedelta(minutes=1), 25), _wave(25))


@case("misc/dates_hours", figsize=(8, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2024, 3, 1, 6, 0), timedelta(hours=1), 15), _wave(15))


@case("misc/dates_days")
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2024, 2, 20), timedelta(days=1), 6), _wave(6))


@case("misc/dates_weeks_month_reset", figsize=(8, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2024, 2, 20), timedelta(days=1), 40), _wave(40))


@case("misc/dates_months", figsize=(6.4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2023, 11, 15), timedelta(days=7), 25), _wave(25))


@case("misc/dates_years", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2015, 1, 1), timedelta(days=60), 40), _wave(40))


@case("misc/dates_decades", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([datetime(1900 + 4 * i, 7, 1) for i in range(30)], _wave(30))


@case("misc/dates_concise_formatter", figsize=(6, 4))
def _(fig):
    axs = fig.subplots(3, 1)
    spans = [(datetime(2024, 1, 30, 22), timedelta(minutes=10)),
             (datetime(2024, 1, 25), timedelta(hours=8)),
             (datetime(2023, 10, 1), timedelta(days=9))]
    for ax, (start, step) in zip(axs, spans):
        locator = mdates.AutoDateLocator()
        ax.xaxis.set_major_locator(locator)
        ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(locator))
        ax.plot(_dates(start, step, 30), _wave(30))
    fig.subplots_adjust(hspace=0.6)


@case("misc/dates_autofmt_xdate", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(_dates(datetime(2024, 5, 1), timedelta(days=3), 30), _wave(30))
    fig.autofmt_xdate()


@case("misc/dates_datetime64_bar", figsize=(8, 3))
def _(fig):
    ax = fig.subplots()
    x = np.arange(np.datetime64("2024-01-01"), np.datetime64("2024-01-07"), np.timedelta64(1, "D"))
    ax.bar(x, [3, 1, 4, 1, 5, 2], width=0.8)


@case("misc/dates_datetime64_locator_formatter", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    x = np.arange(np.datetime64("2024-01-01T00:00"), np.datetime64("2024-01-03T00:00"),
                  np.timedelta64(2, "h"))
    ax.plot(x, _wave(len(x)), "o-")
    ax.xaxis.set_major_locator(mdates.HourLocator(byhour=[0, 12]))
    ax.xaxis.set_major_formatter(mdates.DateFormatter("%d %b %H:%M"))
    ax.xaxis.set_minor_locator(mdates.HourLocator(interval=3))


# --- categorical axes

@case("misc/categorical_plot", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(["apple", "banana", "cherry", "date"], [3, 1, 4, 2], "o-")


@case("misc/categorical_bar_scatter_y", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.bar(["a", "b", "c"], [5, 2, 7], color=["C0", "C1", "C2"])
    ax2.scatter([1, 5, 3, 2], ["low", "high", "mid", "low"])


@case("misc/categorical_mixed_updates", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(["a", "b", "c"], [1, 2, 3])
    ax.plot(["c", "d", "a", "e"], [3, 1, 2, 2], "s--")


# --- log / symlog helpers

@case("misc/semilogx_base2", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = 2.0 ** np.arange(0, 11)
    ax.semilogx(x, np.sqrt(x), "o-", base=2)


@case("misc/semilogy_default", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 5.0, 50)
    ax.semilogy(x, np.exp(2 * x))


@case("misc/loglog_mixed_bases", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.geomspace(1e-2, 1e3, 40)
    ax.loglog(x, x**1.5)
    ax.set_yscale("log", base=2)


@case("misc/symlog_linthresh", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-100.0, 100.0, 401)
    ax.plot(x, x**3)
    ax.set_xscale("symlog", linthresh=1.0)
    ax.set_yscale("symlog", linthresh=10.0, linscale=0.5)


# --- spectral analysis

@case("misc/specgram", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    t = np.arange(2048) / 200.0
    x = np.sin(2 * np.pi * (5 + 4 * t) * t) + 0.2 * rng(62).normal(2048)
    ax.specgram(x, NFFT=128, Fs=200, noverlap=64)


@case("misc/psd", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.psd(_signal(), NFFT=256, Fs=100)


@case("misc/csd", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.csd(_signal(seed=63), _signal(seed=64), NFFT=128, Fs=100)


@case("misc/cohere", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.cohere(_signal(seed=65), _signal(seed=66), NFFT=128, Fs=100)


@case("misc/spectrum_magnitude_angle_phase", figsize=(5, 6))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(3, 1)
    x = _signal(256)
    ax1.magnitude_spectrum(x, Fs=100, scale="dB")
    ax2.angle_spectrum(x, Fs=100)
    ax3.phase_spectrum(x, Fs=100)
    fig.subplots_adjust(hspace=0.7)


@case("misc/acorr_xcorr", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = _signal(200, seed=67)
    ax1.acorr(x, maxlags=30)
    ax2.xcorr(x, _signal(200, seed=68), usevlines=False, maxlags=20, marker="o", ms=3)


# --- Sankey

@case("misc/sankey_simple", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    ax.set_yticks([])
    Sankey(ax=ax, unit=None).add(
        flows=[0.25, 0.15, 0.60, -0.20, -0.15, -0.05, -0.50, -0.10],
        labels=["", "", "", "First", "Second", "Third", "Fourth", "Fifth"],
        orientations=[-1, 1, 0, 1, 1, 1, 0, -1]).finish()


@case("misc/sankey_connected", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    ax.set_yticks([])
    sankey = Sankey(ax=ax, scale=0.01, offset=0.2, format="%.0f", unit="%")
    sankey.add(flows=[25, 0, 60, -10, -20, -5, -15, -10, -40],
               labels=["", "", "", "First", "Second", "Third", "Fourth", "Fifth", "Hurray!"],
               orientations=[-1, 1, 0, 1, 1, 1, -1, -1, 0],
               pathlengths=[0.25, 0.25, 0.25, 0.25, 0.25, 0.6, 0.25, 0.25, 0.25],
               patchlabel="Widget\nA", fc="tab:blue", alpha=0.4)
    sankey.add(flows=[40, -20, -20], orientations=[0, 1, -1], prior=0, connect=(8, 0),
               labels=[None, "out A", "out B"], fc="tab:orange", alpha=0.4)
    sankey.finish()


# --- insets

@case("misc/indicate_inset_zoom", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_field(40), extent=(-3, 3, -3, 3), origin="lower")
    axins = ax.inset_axes((0.6, 0.6, 0.37, 0.37), xlim=(-1.5, -0.5), ylim=(-0.5, 0.5))
    axins.imshow(_field(40), extent=(-3, 3, -3, 3), origin="lower")
    ax.indicate_inset_zoom(axins, edgecolor="black")


@case("misc/indicate_inset_bounds", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(_wave(50))
    ax.indicate_inset((10, 0.0, 15, 1.0), edgecolor="tab:red", alpha=1.0)


# --- axes_grid1 and axisartist

@case("misc/axes_grid1_append_axes", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_field())
    divider = make_axes_locatable(ax)
    cax = divider.append_axes("right", size="5%", pad=0.05)
    fig.colorbar(im, cax=cax)
    top = divider.append_axes("top", size=0.6, pad=0.1, sharex=ax)
    top.plot(_field()[10])
    top.tick_params(labelbottom=False)


@case("misc/imagegrid_single_cbar", figsize=(5, 4))
def _(fig):
    grid = ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1, cbar_mode="single",
                     cbar_location="right", cbar_pad=0.1)
    for i, ax in enumerate(grid):
        im = ax.imshow(_field() * (i + 1) / 4, vmin=-1, vmax=1)
    grid.cbar_axes[0].colorbar(im)


@case("misc/axesgrid_each_cbar_label_mode", figsize=(6, 3))
def _(fig):
    grid = AxesGrid(fig, 111, nrows_ncols=(1, 3), axes_pad=0.5, label_mode="L",
                    cbar_mode="each", cbar_location="top", cbar_pad=0.05)
    for i, (ax, cax) in enumerate(zip(grid, grid.cbar_axes)):
        im = ax.imshow(_field(10) * (i + 1), cmap=["viridis", "magma", "cividis"][i])
        cax.colorbar(im)


@case("misc/axisartist_floating_axis", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis["y=0"] = ax.new_floating_axis(nth_coord=0, value=0)
    ax.axis["x=0"] = ax.new_floating_axis(nth_coord=1, value=0)
    ax.axis["top", "right"].set_visible(False)
    x = np.linspace(-2.0, 2.0, 50)
    ax.plot(x, x**3 - 2 * x)
    ax.set_ylim(-3, 3)


@case("misc/axisartist_axeszero_arrows", figsize=(4, 4))
def _(fig):
    ax = fig.add_subplot(axes_class=axisartist.axislines.AxesZero)
    for direction in ["xzero", "yzero"]:
        ax.axis[direction].set_axisline_style("-|>")
        ax.axis[direction].set_visible(True)
    for direction in ["left", "right", "bottom", "top"]:
        ax.axis[direction].set_visible(False)
    x = np.linspace(-0.5, 1.0, 50)
    ax.plot(x, np.sin(x * np.pi))


# --- figure artists and property cycles

@case("misc/fig_add_artist", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1])
    fig.add_artist(lines.Line2D([0.05, 0.95], [0.5, 0.5], color="tab:red", lw=3))
    fig.add_artist(patches.Circle((0.5, 0.5), 0.1, fc="none", ec="k", lw=2,
                                  transform=fig.transFigure))
    fig.add_artist(patches.Rectangle((10, 10), 50, 30, transform=None, fc="gold"))


@case("misc/set_prop_cycle", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_prop_cycle(color=["tab:red", "tab:green", "k"], linestyle=["-", "--", ":"],
                      linewidth=[1, 2, 3])
    for i in range(5):
        ax.plot(_wave(20) + i)


def _fade_and_shift(image, dpi):
    """Inverted colours at half opacity, dpi/10 to the right and dpi/20 down."""
    out = image.copy()
    out[..., :3] = 1 - out[..., :3]
    out[..., 3] = out[..., 3] * 0.5
    return out, dpi / 10, -dpi / 20


def _box_blur(image, dpi):
    """A separable 3-pixel box blur over a 3-pixel transparent border."""
    padded = np.pad(image, ((3, 3), (3, 3), (0, 0)))
    rows = (padded[:-2] + padded[1:-1] + padded[2:]) / 3
    blurred = (rows[:, :-2] + rows[:, 1:-1] + rows[:, 2:]) / 3
    return blurred, -2, -2


@case("misc/agg_filter", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 2 * np.pi, 50)
    shadow, = ax.plot(x, np.sin(x), lw=4, color="tab:blue")
    shadow.set_agg_filter(_fade_and_shift)
    ax.plot(x, np.sin(x), lw=2, color="tab:blue")
    for bar in ax.bar([1, 3, 5], [0.5, -0.4, 0.8], color="tab:orange"):
        bar.set_agg_filter(_box_blur)
    ax.text(3, -0.8, "filtered", size=14).set_agg_filter(_box_blur)


# --- widgets (initial rendering only)

@case("misc/widgets_sliders_button", figsize=(5, 3))
def _(fig):
    s1 = widgets.Slider(fig.add_axes((0.2, 0.8, 0.6, 0.08)), "Freq", 0.1, 30.0, valinit=5.0)
    s2 = widgets.Slider(fig.add_axes((0.05, 0.1, 0.05, 0.6)), "Amp", 0, 10, valinit=7,
                        valstep=1, orientation="vertical")
    s3 = widgets.RangeSlider(fig.add_axes((0.2, 0.6, 0.6, 0.08)), "Range", 0.0, 1.0,
                             valinit=(0.2, 0.7))
    b = widgets.Button(fig.add_axes((0.6, 0.15, 0.25, 0.15)), "Reset", color="0.9")
    del s1, s2, s3, b


@case("misc/widgets_check_radio_textbox", figsize=(6, 3))
def _(fig):
    c = widgets.CheckButtons(fig.add_axes((0.05, 0.3, 0.25, 0.5)), ["red", "green", "blue"],
                             [True, False, True])
    r = widgets.RadioButtons(fig.add_axes((0.38, 0.3, 0.25, 0.5)), ["one", "two", "three"],
                             active=1)
    t = widgets.TextBox(fig.add_axes((0.78, 0.45, 0.18, 0.1)), "Name ", initial="plt-rs")
    del c, r, t
