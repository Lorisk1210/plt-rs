"""The gallery's "Ticks", "Scales", "Units" and "Color" sections
(``galleries/examples/{ticks,scales,units,color}``), one case per figure
(``<example>_<n>`` for examples drawing several). The units examples use
the gallery's ``basic_units`` module, copied as ``figures/basic_units``.

The examples run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng`` (multivariate normals through the Cholesky
factor of their covariance), and ``goog.npz`` from a synthetic series of
the same shape: 1047 weekdays from 2004-08-19 with a random walk of
adjusted closing prices (``_goog``). ``date_formatters_locators`` builds
its locators and formatters directly instead of ``eval``-ing their names;
``date_concise_formatter`` restores the unit registry once its converters
have converted its dates, and ``custom_cmap`` registers its colormaps only
once. ``date_index_formatter`` installs only
its final formatter (the ``MyFormatter`` class), which replaces the
function formatter set before it.

``date_precision_and_epochs`` is left out: it plots microseconds 30 years
from the epoch, where the data transform scales date numbers by 4.6e11
pixels a day and cancels them against an offset of 5e15 pixels, so the
line and tick label positions are the rounding noise of the
constrained-layout solution (kiwisolver's) and shift by up to a pixel with
any one-ulp difference in it."""

import datetime
import math

import matplotlib as mpl
import matplotlib.units as munits
import numpy as np
from matplotlib import dates as mdates
from matplotlib import lines as mlines
from matplotlib import patches as mpatches
from matplotlib import scale as mscale
from matplotlib import text as mtext
from matplotlib import ticker
from matplotlib import transforms as mtransforms
from matplotlib.colors import (TABLEAU_COLORS, BoundaryNorm, LinearSegmentedColormap,
                               PowerNorm, same_color)
from matplotlib.patches import Rectangle

from spec_support import case, rng

from . import basic_units
from .basic_units import cm, degrees, hertz, inch, minutes, radians, secs

G = "gallery_ticks"
SEED = 19680801


def gallery(name, **options):
    """Registers a case of this group."""
    return case(f"{G}/{name}", **options)


def _goog(n=1047):
    """A synthetic stand-in for ``goog.npz``'s ``price_data``: ``n``
    trading days (weekdays) from 2004-08-19 and a random walk of adjusted
    closing prices, as a record array with ``date`` and ``adj_close``."""
    dates = np.busday_offset(np.datetime64("2004-08-19"), np.arange(n), roll="forward")
    adj_close = 100 * np.exp(np.cumsum(rng(2004).normal(n, 0.002, 0.02)))
    return np.rec.fromarrays([dates, adj_close], names="date,adj_close")


# --- ticks: labels ---------------------------------------------------------------


@gallery("align_ticklabels")
def _(fig):
    population = {
        "Sydney": 5.2,
        "Mexico City": 8.8,
        "São Paulo": 12.2,
        "Istanbul": 15.9,
        "Lagos": 15.9,
        "Shanghai": 21.9,
    }
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.barh(population.keys(), population.values())
    ax.set_xlabel('Population (in millions)')
    for ticklabel in ax.get_yticklabels():
        ticklabel.set_horizontalalignment("left")
    ax.tick_params("y", pad=70)


@gallery("ticklabels_rotation")
def _(fig):
    population = {
        'India': 1417.492,
        'China': 1404.89,
        'United States': 341.784857,
        'Indonesia': 284.438782,
        'Pakistan': 241.499431,
        'Nigeria': 223.8,
        'Brazil': 213.421037,
        'Bangladesh': 169.828911,
        'Russia': 146.028325,
        'Mexico': 131.001723,
        'Japan': 122.95,
        'Philippines': 114.1236,
        'DR Congo': 112.832,
        'Ethiopia': 111.652998,
        'Egypt': 107.868296,
        'Vietnam': 102.3,
    }
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.bar(population.keys(), population.values())
    ax.tick_params("x", rotation=45, rotation_mode="xtick")
    ax.set_ylabel("population (millions)")


@gallery("ticks_top_right")
def _(fig):
    ax = fig.subplots()
    ax.plot(range(10))
    ax.tick_params(
        top=True, labeltop=True,
        bottom=False, labelbottom=False,
        right=True, labelright=True,
        left=False, labelleft=False,
    )
    ax.xaxis.set_label_position("top")
    ax.yaxis.set_label_position("right")
    ax.set_xlabel("X label")
    ax.set_ylabel("Y label")


@gallery("fig_axes_customize_simple")
def _(fig):
    fig.patch.set_facecolor('lightgoldenrodyellow')
    ax1 = fig.add_axes((0.1, 0.3, 0.4, 0.4))
    ax1.patch.set_facecolor('lightslategray')
    ax1.tick_params(axis='x', labelcolor='tab:red', labelrotation=45, labelsize=16)
    ax1.tick_params(axis='y', color='tab:green', size=25, width=3)


@gallery("multilevel_ticks_1", figsize=(4, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.plot(np.arange(30))
    sec = ax.secondary_xaxis(location=0)
    sec.set_xticks([5, 15, 25], labels=['\nOughts', '\nTeens', '\nTwenties'])


@gallery("multilevel_ticks_2", figsize=(7, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.plot(['cats', 'dogs', 'pigs', 'snakes', 'lizards', 'chickens',
             'eagles', 'herons', 'buzzards'],
            rng(SEED).normal(9), 'o')
    sec = ax.secondary_xaxis(location=0)
    sec.set_xticks([1, 3.5, 6.5], labels=['\n\nMammals', '\n\nReptiles', '\n\nBirds'])
    sec.tick_params('x', length=0)
    sec2 = ax.secondary_xaxis(location=0)
    sec2.set_xticks([-0.5, 2.5, 4.5, 8.5], labels=[])
    sec2.tick_params('x', length=40, width=1.5)
    ax.set_xlim(-0.6, 8.6)


@gallery("multilevel_ticks_3", figsize=(7, 4))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    time = np.arange(np.datetime64('2020-01-01'), np.datetime64('2020-03-31'),
                     np.timedelta64(1, 'D'))
    ax.plot(time, rng(SEED).uniform(len(time)))
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%d'))
    sec = ax.secondary_xaxis(location=-0.075)
    sec.xaxis.set_major_locator(mdates.MonthLocator(bymonthday=1))
    sec.xaxis.set_major_formatter(mdates.DateFormatter('  %b'))
    sec.tick_params('x', length=0)
    sec.spines['bottom'].set_linewidth(0)
    sec.set_xlabel('Dates (2020)')


def _ticks_out_of_order(fig, x, y, *fmt):
    """The left axes of ``ticks_too_many``: string values plotted as
    categories, their ticks in red."""
    ax = fig.subplots(1, 2)
    ax[0].plot(x, y, *fmt)
    ax[0].tick_params(axis='x', color='r', labelcolor='r')
    return ax


@gallery("ticks_too_many_1", figsize=(6, 2.5))
def _(fig):
    fig.set_layout_engine("constrained")
    x = ['1', '5', '2', '3']
    y = [1, 4, 2, 3]
    ax = _ticks_out_of_order(fig, x, y, 'd')
    ax[0].set_xlabel('Categories')
    ax[0].set_title('Ticks seem out of order / misplaced')
    x = np.asarray(x, dtype='float')
    ax[1].plot(x, y, 'd')
    ax[1].set_xlabel('Floats')
    ax[1].set_title('Ticks as expected')


@gallery("ticks_too_many_2", figsize=(6, 2.5))
def _(fig):
    x = [f'{xx}' for xx in np.arange(100)]
    y = np.arange(100)
    ax = _ticks_out_of_order(fig, x, y)
    ax[0].set_title('Too many ticks')
    ax[0].set_xlabel('Categories')
    ax[1].plot(np.asarray(x, float), y)
    ax[1].set_title('x converted to numbers')
    ax[1].set_xlabel('Floats')


@gallery("ticks_too_many_3", figsize=(6, 2.75))
def _(fig):
    fig.set_layout_engine("constrained")
    x = ['2021-10-01', '2021-11-02', '2021-12-03', '2021-09-01']
    y = [0, 2, 3, 1]
    ax = _ticks_out_of_order(fig, x, y, 'd')
    ax[0].tick_params(axis='x', labelrotation=90)
    ax[0].set_title('Dates out of order')
    x = np.asarray(x, dtype='datetime64[s]')
    ax[1].plot(x, y, 'd')
    ax[1].tick_params(axis='x', labelrotation=90)
    ax[1].set_title('x converted to datetimes')


# --- ticks: locators and formatters ---------------------------------------------


def _auto_ticks_scatter(fig):
    """The ``auto_ticks`` scatter of a 10 x 10 grid over [0.3, 1.2]."""
    ax = fig.subplots()
    dots = np.linspace(0.3, 1.2, 10)
    X, Y = np.meshgrid(dots, dots)
    x, y = X.ravel(), Y.ravel()
    ax.scatter(x, y, c=x+y)
    return ax


@gallery("auto_ticks_1")
def _(fig):
    _auto_ticks_scatter(fig)


@gallery("auto_ticks_2", rc={"axes.autolimit_mode": "round_numbers"})
def _(fig):
    _auto_ticks_scatter(fig)


@gallery("auto_ticks_3", rc={"axes.autolimit_mode": "round_numbers"})
def _(fig):
    _auto_ticks_scatter(fig).set_xmargin(0.8)


@gallery("custom_ticker1")
def _(fig):
    def millions(x, pos):
        """The two arguments are the value and tick position."""
        return f'${x*1e-6:1.1f}M'

    ax = fig.subplots()
    ax.yaxis.set_major_formatter(millions)
    money = [1.5e5, 2.5e6, 5.5e6, 2.0e7]
    ax.bar(['Bill', 'Fred', 'Mary', 'Sue'], money)


@gallery("dollar_ticks")
def _(fig):
    ax = fig.subplots()
    ax.plot(100*rng(SEED).uniform(20))
    ax.yaxis.set_major_formatter('${x:1.2f}')
    ax.yaxis.set_tick_params(which='major', labelcolor='green',
                             labelleft=False, labelright=True)


@gallery("engformatter_offset")
def _(fig):
    ax = fig.subplots()
    ax.yaxis.set_major_formatter(ticker.EngFormatter(useOffset=True, unit="Hz"))
    size = 100
    measurement = np.full(size, 1e9)
    noise = rng(SEED).uniform(size, -2e3, 2e3)
    ax.plot(measurement + noise)


@gallery("engineering_formatter", figsize=(7, 9.6))
def _(fig):
    xs = np.logspace(1, 9, 100)
    ys = (0.8 + 0.4 * rng(SEED).uniform(100)) * np.log10(xs)**2
    ax0, ax1 = fig.subplots(nrows=2)
    for ax in (ax0, ax1):
        ax.set_xscale('log')
    ax0.set_title('Full unit ticklabels, w/ default precision & space separator')
    ax0.xaxis.set_major_formatter(ticker.EngFormatter(unit='Hz'))
    ax0.plot(xs, ys)
    ax0.set_xlabel('Frequency')
    ax1.set_title('SI-prefix only ticklabels, 1-digit precision & '
                  'thin space separator')
    ax1.xaxis.set_major_formatter(ticker.EngFormatter(places=1, sep="\N{THIN SPACE}"))
    ax1.plot(xs, ys)
    ax1.set_xlabel('Frequency [Hz]')
    fig.tight_layout()


@gallery("major_minor_demo_1")
def _(fig):
    t = np.arange(0.0, 100.0, 0.1)
    s = np.sin(0.1 * np.pi * t) * np.exp(-t * 0.01)
    ax = fig.subplots()
    ax.plot(t, s)
    ax.xaxis.set_major_locator(ticker.MultipleLocator(20))
    ax.xaxis.set_major_formatter('{x:.0f}')
    ax.xaxis.set_minor_locator(ticker.MultipleLocator(5))


@gallery("major_minor_demo_2")
def _(fig):
    t = np.arange(0.0, 100.0, 0.01)
    s = np.sin(2 * np.pi * t) * np.exp(-t * 0.01)
    ax = fig.subplots()
    ax.plot(t, s)
    ax.xaxis.set_minor_locator(ticker.AutoMinorLocator())
    ax.tick_params(which='both', width=2)
    ax.tick_params(which='major', length=7)
    ax.tick_params(which='minor', length=4, color='r')


@gallery("scalarformatter", figsize=(9, 9))
def _(fig):
    x = np.arange(0, 1, .01)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 3, gridspec_kw={"hspace": 0.1})
    for col in axs.T:
        col[0].plot(x * 1e5 + 1e10, x * 1e-10 + 1e-5)
        col[1].plot(x * 1e5, x * 1e-4)
        col[2].plot(-x * 1e5 - 1e10, -x * 1e-5 - 1e-10)
    for ax in axs[:, 1]:
        ax.ticklabel_format(useMathText=True)
    for ax in axs[:, 2]:
        ax.ticklabel_format(useOffset=False)
    mpl.rcParams.update({"axes.titleweight": "bold", "axes.titley": 1.1})
    axs[0, 0].set_title("default settings")
    axs[0, 1].set_title("useMathText=True")
    axs[0, 2].set_title("useOffset=False")


@gallery("tick_labels_from_values")
def _(fig):
    ax = fig.subplots()
    xs = range(26)
    ys = range(26)
    labels = list('abcdefghijklmnopqrstuvwxyz')

    def format_fn(tick_val, tick_pos):
        if int(tick_val) in xs:
            return labels[int(tick_val)]
        else:
            return ''

    ax.xaxis.set_major_formatter(format_fn)
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax.plot(xs, ys)


def _strip(ax, title):
    """``setup`` of ``tick-locators`` and ``tick-formatters``: only the
    bottom spine and the x ticks over [0, 5], ``title`` in monospace."""
    ax.yaxis.set_major_locator(ticker.NullLocator())
    ax.spines[['left', 'right', 'top']].set_visible(False)
    ax.xaxis.set_ticks_position('bottom')
    ax.tick_params(which='major', width=1.00, length=5)
    ax.tick_params(which='minor', width=0.75, length=2.5)
    ax.set_xlim(0, 5)
    ax.set_ylim(0, 1)
    ax.text(0.0, 0.2, title, transform=ax.transAxes,
            fontsize=14, fontname='Monospace', color='tab:blue')


@gallery("tick_locators", figsize=(8, 6))
def _(fig):
    axs = fig.subplots(8, 1)
    _strip(axs[0], title="NullLocator()")
    axs[0].xaxis.set_major_locator(ticker.NullLocator())
    axs[0].xaxis.set_minor_locator(ticker.NullLocator())
    _strip(axs[1], title="MultipleLocator(0.5, offset=0.2)")
    axs[1].xaxis.set_major_locator(ticker.MultipleLocator(0.5, offset=0.2))
    axs[1].xaxis.set_minor_locator(ticker.MultipleLocator(0.1))
    _strip(axs[2], title="FixedLocator([0, 1, 5])")
    axs[2].xaxis.set_major_locator(ticker.FixedLocator([0, 1, 5]))
    axs[2].xaxis.set_minor_locator(ticker.FixedLocator(np.linspace(0.2, 0.8, 4)))
    _strip(axs[3], title="LinearLocator(numticks=3)")
    axs[3].xaxis.set_major_locator(ticker.LinearLocator(3))
    axs[3].xaxis.set_minor_locator(ticker.LinearLocator(31))
    _strip(axs[4], title="IndexLocator(base=0.5, offset=0.25)")
    axs[4].plot([0]*5, color='white')
    axs[4].xaxis.set_major_locator(ticker.IndexLocator(base=0.5, offset=0.25))
    _strip(axs[5], title="AutoLocator()")
    axs[5].xaxis.set_major_locator(ticker.AutoLocator())
    axs[5].xaxis.set_minor_locator(ticker.AutoMinorLocator())
    _strip(axs[6], title="MaxNLocator(n=4)")
    axs[6].xaxis.set_major_locator(ticker.MaxNLocator(4))
    axs[6].xaxis.set_minor_locator(ticker.MaxNLocator(40))
    _strip(axs[7], title="LogLocator(base=10, numticks=15)")
    axs[7].set_xlim(10**3, 10**10)
    axs[7].set_xscale('log')
    axs[7].xaxis.set_major_locator(ticker.LogLocator(base=10, numticks=15))
    fig.tight_layout()


def _formatter_strip(ax, title):
    """``setup`` of ``tick-formatters``: the strip with major ticks at
    whole numbers and labelled minor ticks at quarters."""
    _strip(ax, title)
    ax.xaxis.set_major_locator(ticker.MultipleLocator(1.00))
    ax.xaxis.set_minor_locator(ticker.MultipleLocator(0.25))
    ax.tick_params(which='minor', labelsize=10)


@gallery("tick_formatters", figsize=(8, 8))
def _(fig):
    fig.set_layout_engine("constrained")
    fig0, fig1, fig2 = fig.subfigures(3, height_ratios=[1.5, 1.5, 7.5])

    fig0.suptitle('String Formatting', fontsize=16, x=0, ha='left')
    ax0 = fig0.subplots()
    _formatter_strip(ax0, title="'{x} km'")
    ax0.xaxis.set_major_formatter('{x} km')

    fig1.suptitle('Function Formatting', fontsize=16, x=0, ha='left')
    ax1 = fig1.subplots()
    _formatter_strip(ax1, title="def(x, pos): return str(x-5)")
    ax1.xaxis.set_major_formatter(lambda x, pos: str(x-5))

    fig2.suptitle('Formatter Object Formatting', fontsize=16, x=0, ha='left')
    axs2 = fig2.subplots(7, 1)
    _formatter_strip(axs2[0], title="NullFormatter()")
    axs2[0].xaxis.set_major_formatter(ticker.NullFormatter())
    _formatter_strip(axs2[1], title="StrMethodFormatter('{x:.3f}')")
    axs2[1].xaxis.set_major_formatter(ticker.StrMethodFormatter("{x:.3f}"))
    _formatter_strip(axs2[2], title="FormatStrFormatter('#%d')")
    axs2[2].xaxis.set_major_formatter(ticker.FormatStrFormatter("#%d"))

    def fmt_two_digits(x, pos):
        return f'[{x:.2f}]'

    _formatter_strip(axs2[3], title='FuncFormatter("[{:.2f}]".format)')
    axs2[3].xaxis.set_major_formatter(ticker.FuncFormatter(fmt_two_digits))
    _formatter_strip(axs2[4], title="FixedFormatter(['A', 'B', 'C', 'D', 'E', 'F'])")
    axs2[4].xaxis.set_major_locator(ticker.FixedLocator([0, 1, 2, 3, 4, 5]))
    axs2[4].xaxis.set_major_formatter(ticker.FixedFormatter(['A', 'B', 'C', 'D', 'E', 'F']))
    _formatter_strip(axs2[5], title="ScalarFormatter()")
    axs2[5].xaxis.set_major_formatter(ticker.ScalarFormatter(useMathText=True))
    _formatter_strip(axs2[6], title="PercentFormatter(xmax=5)")
    axs2[6].xaxis.set_major_formatter(ticker.PercentFormatter(xmax=5))


@gallery("colorbar_tick_labelling_demo_1")
def _(fig):
    ax = fig.subplots()
    data = rng(SEED).normal(250 * 250).reshape(250, 250)
    cax = ax.imshow(data, vmin=-1, vmax=1, cmap='coolwarm')
    ax.set_title('Gaussian noise with vertical colorbar')
    cbar = fig.colorbar(cax, ticks=[-1, 0, 1],
                        format=ticker.FixedFormatter(['< -1', '0', '> 1']),
                        extend='both')
    labels = cbar.ax.get_yticklabels()
    labels[0].set_verticalalignment('top')
    labels[-1].set_verticalalignment('bottom')


@gallery("colorbar_tick_labelling_demo_2")
def _(fig):
    ax = fig.subplots()
    data = np.clip(rng(SEED).normal(250 * 250).reshape(250, 250), -1, 1)
    cax = ax.imshow(data, cmap='afmhot')
    ax.set_title('Gaussian noise with horizontal colorbar')
    cbar = fig.colorbar(cax, orientation='horizontal')
    cbar.set_ticks(ticks=[-1, 0, 1], labels=['Low', 'Medium', 'High'])


# --- ticks: dates ----------------------------------------------------------------


@gallery("centered_ticklabels")
def _(fig):
    r = _goog()[-250:]
    ax = fig.subplots()
    ax.plot(r["date"], r["adj_close"])
    ax.xaxis.set_major_locator(mdates.MonthLocator())
    ax.xaxis.set_minor_locator(mdates.MonthLocator(bymonthday=16))
    ax.xaxis.set_major_formatter(ticker.NullFormatter())
    ax.xaxis.set_minor_formatter(mdates.DateFormatter('%b'))
    ax.tick_params(axis='x', which='minor', tick1On=False, tick2On=False)
    imid = len(r) // 2
    ax.set_xlabel(str(r["date"][imid].item().year))


@gallery("date", figsize=(6.4, 7))
def _(fig):
    data = _goog()
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 1)
    for ax in axs:
        ax.plot('date', 'adj_close', data=data)
        ax.xaxis.set_major_locator(mdates.MonthLocator(bymonth=(1, 7)))
        ax.xaxis.set_minor_locator(mdates.MonthLocator())
        ax.grid(True)
        ax.set_ylabel(r'Price [\$]')
    ax = axs[0]
    ax.set_title('DefaultFormatter', loc='left', y=0.85, x=0.02, fontsize='medium')
    ax = axs[1]
    ax.set_title('ConciseFormatter', loc='left', y=0.85, x=0.02, fontsize='medium')
    ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(ax.xaxis.get_major_locator()))
    ax = axs[2]
    ax.set_title('Manual DateFormatter', loc='left', y=0.85, x=0.02, fontsize='medium')
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%b'))
    ax.xaxis.set_tick_params(rotation=30, rotation_mode='xtick')


@gallery("date_demo_convert")
def _(fig):
    date1 = datetime.datetime(2000, 3, 2)
    date2 = datetime.datetime(2000, 3, 6)
    delta = datetime.timedelta(hours=6)
    dates = mdates.drange(date1, date2, delta)
    y = np.arange(len(dates))
    ax = fig.subplots()
    ax.plot(dates, y**2, 'o')
    ax.set_xlim(dates[0], dates[-1])
    ax.xaxis.set_major_locator(mdates.DayLocator())
    ax.xaxis.set_minor_locator(mdates.HourLocator(range(0, 25, 6)))
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%Y-%m-%d'))
    ax.fmt_xdata = mdates.DateFormatter('%Y-%m-%d %H:%M:%S')
    fig.autofmt_xdate()


@gallery("date_demo_rrule")
def _(fig):
    rule = mdates.rrulewrapper(mdates.YEARLY, byeaster=1, interval=5)
    loc = mdates.RRuleLocator(rule)
    formatter = mdates.DateFormatter('%m/%d/%y')
    date1 = datetime.date(1952, 1, 1)
    date2 = datetime.date(2004, 4, 12)
    delta = datetime.timedelta(days=100)
    dates = mdates.drange(date1, date2, delta)
    s = rng(SEED).uniform(len(dates))
    ax = fig.subplots()
    ax.plot(dates, s, 'o')
    ax.xaxis.set_major_locator(loc)
    ax.xaxis.set_major_formatter(formatter)
    ax.xaxis.set_tick_params(rotation=30, labelsize=10)


def _date_strip(ax, title, xmax='2002-02-01'):
    """``plot_axis`` of ``date_formatters_locators`` before its locator or
    formatter is set: a bare strip from 2000-02-01 to ``xmax``."""
    ax.spines[['left', 'right', 'top']].set_visible(False)
    ax.yaxis.set_major_locator(ticker.NullLocator())
    ax.tick_params(which='major', width=1.00, length=5)
    ax.tick_params(which='minor', width=0.75, length=2.5)
    ax.set_xlim(np.datetime64('2000-02-01'), np.datetime64(xmax))
    ax.text(0.0, 0.2, title, transform=ax.transAxes,
            fontsize=14, fontname='Monospace', color='tab:blue')


DATE_LOCATORS = [
    ('AutoDateLocator(maxticks=8)', lambda: mdates.AutoDateLocator(maxticks=8),
     '2003-02-01', '%Y-%m'),
    ('YearLocator(month=4)', lambda: mdates.YearLocator(month=4), '2003-02-01', '%Y-%m'),
    ('MonthLocator(bymonth=[4, 8, 12])', lambda: mdates.MonthLocator(bymonth=[4, 8, 12]),
     '2003-02-01', '%Y-%m'),
    ('DayLocator(interval=180)', lambda: mdates.DayLocator(interval=180),
     '2003-02-01', '%Y-%m-%d'),
    ('WeekdayLocator(byweekday=SU, interval=4)',
     lambda: mdates.WeekdayLocator(byweekday=mdates.SU, interval=4),
     '2000-07-01', '%a %Y-%m-%d'),
    ('HourLocator(byhour=range(0, 24, 6))', lambda: mdates.HourLocator(byhour=range(0, 24, 6)),
     '2000-02-04', '%H h'),
    ('MinuteLocator(interval=15)', lambda: mdates.MinuteLocator(interval=15),
     '2000-02-01 02:00', '%H:%M'),
    ('SecondLocator(bysecond=(0, 30))', lambda: mdates.SecondLocator(bysecond=(0, 30)),
     '2000-02-01 00:02', '%H:%M:%S'),
    ('MicrosecondLocator(interval=1000)', lambda: mdates.MicrosecondLocator(interval=1000),
     '2000-02-01 00:00:00.005', '%S.%f'),
    ('RRuleLocator(rrulewrapper(freq=MONTHLY, \nbyweekday=(MO, TU, WE, TH, FR), '
     'bysetpos=-1))',
     lambda: mdates.RRuleLocator(mdates.rrulewrapper(
         freq=mdates.MONTHLY,
         byweekday=(mdates.MO, mdates.TU, mdates.WE, mdates.TH, mdates.FR),
         bysetpos=-1)),
     '2000-07-01', '%Y-%m-%d'),
]


@gallery("date_formatters_locators_1", figsize=(8, len(DATE_LOCATORS) * .8))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(len(DATE_LOCATORS), 1)
    fig.suptitle('Date Locators')
    for ax, (title, locator, xmax, fmt) in zip(axs, DATE_LOCATORS):
        _date_strip(ax, title, xmax)
        ax.xaxis.set_major_locator(locator())
        ax.xaxis.set_major_formatter(mdates.DateFormatter(fmt))


@gallery("date_formatters_locators_2", figsize=(8, 3 * .8))
def _(fig):
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 1)
    fig.suptitle('Date Formatters')
    formatters = [
        ('AutoDateFormatter(ax.xaxis.get_major_locator())',
         lambda ax: mdates.AutoDateFormatter(ax.xaxis.get_major_locator())),
        ('ConciseDateFormatter(ax.xaxis.get_major_locator())',
         lambda ax: mdates.ConciseDateFormatter(ax.xaxis.get_major_locator())),
        ('DateFormatter("%b %Y")', lambda ax: mdates.DateFormatter("%b %Y")),
    ]
    for ax, (title, formatter) in zip(axs, formatters):
        _date_strip(ax, title)
        ax.xaxis.set_major_formatter(formatter(ax))


@gallery("date_index_formatter", figsize=(6, 6))
def _(fig):
    r = _goog()[:9]
    fig.set_layout_engine("constrained")
    fig.get_layout_engine().set(hspace=0.15)
    ax1, ax2 = fig.subplots(nrows=2)
    ax1.plot(r["date"], r["adj_close"], 'o-')
    gaps = np.flatnonzero(np.diff(r["date"]) > np.timedelta64(1, 'D'))
    for gap in r[['date', 'adj_close']][np.stack((gaps, gaps + 1)).T]:
        ax1.plot(gap['date'], gap['adj_close'], 'w--', lw=2)
    ax1.legend(handles=[mlines.Line2D([], [], ls='--', label='Gaps in daily data')])
    ax1.set_title("Plot y at x Coordinates")
    ax1.xaxis.set_major_locator(mdates.DayLocator())
    ax1.xaxis.set_major_formatter(mdates.DateFormatter('%a'))

    class MyFormatter(ticker.Formatter):
        def __init__(self, dates, fmt='%a'):
            self.dates = dates
            self.fmt = fmt

        def __call__(self, x, pos=0):
            """Return the label for time x at position pos."""
            try:
                return self.dates[round(x)].item().strftime(self.fmt)
            except IndexError:
                pass

    ax2.plot(r["adj_close"], 'o-')
    ax2.set_title("Plot y at Index Coordinates Using Custom Formatter")
    ax2.xaxis.set_major_formatter(MyFormatter(r["date"], '%a'))


DATE_LIMITS = [
    (np.datetime64('2005-02'), np.datetime64('2005-04')),
    (np.datetime64('2005-02-03'), np.datetime64('2005-02-15')),
    (np.datetime64('2005-02-03 11:00'), np.datetime64('2005-02-04 13:20')),
]

CONCISE_FORMATS = ['%y', '%b', '%d', '%H:%M', '%H:%M', '%S.%f']
CONCISE_ZERO_FORMATS = ['', '%y', '%b', '%d-%b', '%H:%M', '%H:%M']
CONCISE_OFFSET_FORMATS = ['', '%Y', '%b %Y', '%d %b %Y', '%d %b %Y', '%d %b %Y %H:%M']


def _concise_axes(fig, title, setup=lambda ax: None):
    """The three axes of ``date_concise_formatter``: the same two-hourly
    random walk over two months, twelve days and a day and a bit, each
    axes prepared by ``setup`` before plotting."""
    base = datetime.datetime(2005, 2, 1)
    dates = [base + datetime.timedelta(hours=(2 * i)) for i in range(732)]
    y = np.cumsum(rng(SEED).normal(len(dates)))
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 1)
    for ax, lims in zip(axs, DATE_LIMITS):
        setup(ax)
        ax.plot(dates, y)
        ax.set_xlim(lims)
    axs[0].set_title(title)
    return axs


def _with_date_converter(converter, build):
    """Runs ``build`` with ``converter`` registered for dates, as the
    example does, then restores the registry."""
    kinds = (np.datetime64, datetime.date, datetime.datetime)
    saved = {kind: munits.registry.get(kind) for kind in kinds}
    for kind in kinds:
        munits.registry[kind] = converter
    try:
        build()
    finally:
        munits.registry.update(saved)


@gallery("date_concise_formatter_1", figsize=(6, 6))
def _(fig):
    axs = _concise_axes(fig, 'Default Date Formatter')
    for ax in axs:
        ax.tick_params(axis='x', rotation=40, rotation_mode='xtick')


@gallery("date_concise_formatter_2", figsize=(6, 6))
def _(fig):
    def setup(ax):
        locator = mdates.AutoDateLocator(minticks=3, maxticks=7)
        ax.xaxis.set_major_locator(locator)
        ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(locator))

    _concise_axes(fig, 'Concise Date Formatter', setup)


@gallery("date_concise_formatter_3", figsize=(6, 6))
def _(fig):
    _with_date_converter(mdates.ConciseDateConverter(),
                         lambda: _concise_axes(fig, 'Concise Date Formatter'))


@gallery("date_concise_formatter_4", figsize=(6, 6))
def _(fig):
    def setup(ax):
        locator = mdates.AutoDateLocator()
        formatter = mdates.ConciseDateFormatter(locator)
        formatter.formats = list(CONCISE_FORMATS)
        formatter.zero_formats = [''] + formatter.formats[:-1]
        formatter.zero_formats[3] = '%d-%b'
        formatter.offset_formats = list(CONCISE_OFFSET_FORMATS)
        ax.xaxis.set_major_locator(locator)
        ax.xaxis.set_major_formatter(formatter)

    _concise_axes(fig, 'Concise Date Formatter', setup)


@gallery("date_concise_formatter_5", figsize=(6, 6))
def _(fig):
    converter = mdates.ConciseDateConverter(
        formats=CONCISE_FORMATS, zero_formats=CONCISE_ZERO_FORMATS,
        offset_formats=CONCISE_OFFSET_FORMATS)
    _with_date_converter(converter, lambda: _concise_axes(
        fig, 'Concise Date Formatter registered non-default'))


# --- scales ----------------------------------------------------------------------


@gallery("asinh_demo_1")
def _(fig):
    x = np.linspace(-3, 6, 500)
    ax0, ax1 = fig.subplots(1, 2, sharex=True)
    ax0.plot(x, x)
    ax0.set_yscale('symlog')
    ax0.grid()
    ax0.set_title('symlog')
    ax1.plot(x, x)
    ax1.set_yscale('asinh')
    ax1.grid()
    ax1.set_title('asinh')


@gallery("asinh_demo_2")
def _(fig):
    x = np.linspace(-3, 6, 500)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(1, 3, sharex=True)
    for ax, (a0, base) in zip(axs, ((0.2, 2), (1.0, 0), (5.0, 10))):
        ax.set_title(f'linear_width={a0:.3g}')
        ax.plot(x, x, label='y=x')
        ax.plot(x, 10*x, label='y=10x')
        ax.plot(x, 100*x, label='y=100x')
        ax.set_yscale('asinh', linear_width=a0, base=base)
        ax.grid()
        ax.legend(loc='best', fontsize='small')


@gallery("asinh_demo_3")
def _(fig):
    g = rng(SEED)
    ax = fig.subplots(1, 1)
    r = 3 * np.tan(g.uniform(5000, -np.pi / 2.02, np.pi / 2.02))
    th = g.uniform(len(r), 0, 2*np.pi)
    ax.scatter(r * np.cos(th), r * np.sin(th), s=4, alpha=0.5)
    ax.set_xscale('asinh')
    ax.set_yscale('symlog')
    ax.set_xlabel('asinh')
    ax.set_ylabel('symlog')
    ax.set_title('2D Cauchy random deviates')
    ax.set_xlim(-50, 50)
    ax.set_ylim(-50, 50)
    ax.grid()


@gallery("aspect_loglog")
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.set_xscale("log")
    ax1.set_yscale("log")
    ax1.set_xlim(1e1, 1e3)
    ax1.set_ylim(1e2, 1e3)
    ax1.set_aspect(1)
    ax1.set_title("adjustable = box")
    ax2.set_xscale("log")
    ax2.set_yscale("log")
    ax2.set_adjustable("datalim")
    ax2.plot([1, 3, 10], [1, 9, 100], "o-")
    ax2.set_xlim(1e-1, 1e2)
    ax2.set_ylim(1e-1, 1e3)
    ax2.set_aspect(1)
    ax2.set_title("adjustable = datalim")


class MercatorLatitudeScale(mscale.ScaleBase):
    """``custom_scale``'s scale of latitudes in a Mercator projection,
    ``ln(tan(y) + sec(y))``, cropped at ``thresh``."""

    name = 'mercator'

    def __init__(self, axis, *, thresh=np.deg2rad(85), **kwargs):
        super().__init__(axis)
        if thresh >= np.pi / 2:
            raise ValueError("thresh must be less than pi/2")
        self.thresh = thresh

    def get_transform(self):
        return self.MercatorLatitudeTransform(self.thresh)

    def set_default_locators_and_formatters(self, axis):
        fmt = ticker.FuncFormatter(
            lambda x, pos=None: f"{np.degrees(x):.0f}\N{DEGREE SIGN}")
        axis.set(major_locator=ticker.FixedLocator(np.radians(range(-90, 90, 10))),
                 major_formatter=fmt, minor_formatter=fmt)

    def limit_range_for_scale(self, vmin, vmax, minpos):
        return max(vmin, -self.thresh), min(vmax, self.thresh)

    class MercatorLatitudeTransform(mtransforms.Transform):
        input_dims = output_dims = 1

        def __init__(self, thresh):
            mtransforms.Transform.__init__(self)
            self.thresh = thresh

        def transform_non_affine(self, a):
            masked = np.ma.masked_where((a < -self.thresh) | (a > self.thresh), a)
            if masked.mask.any():
                return np.ma.log(np.abs(np.ma.tan(masked) + 1 / np.ma.cos(masked)))
            else:
                return np.log(np.abs(np.tan(a) + 1 / np.cos(a)))

        def inverted(self):
            return MercatorLatitudeScale.InvertedMercatorLatitudeTransform(self.thresh)

    class InvertedMercatorLatitudeTransform(mtransforms.Transform):
        input_dims = output_dims = 1

        def __init__(self, thresh):
            mtransforms.Transform.__init__(self)
            self.thresh = thresh

        def transform_non_affine(self, a):
            return np.arctan(np.sinh(a))

        def inverted(self):
            return MercatorLatitudeScale.MercatorLatitudeTransform(self.thresh)


mscale.register_scale(MercatorLatitudeScale)


@gallery("custom_scale")
def _(fig):
    t = np.arange(-180.0, 180.0, 0.1)
    s = np.radians(t)/2.
    ax = fig.subplots()
    ax.plot(t, s, '-', lw=2)
    ax.set_yscale('mercator')
    ax.set_xlabel('Longitude')
    ax.set_ylabel('Latitude')
    ax.set_title('Mercator projection')
    ax.grid(True)


def _log_grids(*axs):
    """Major grid lines plus light grey minor ones (``log_demo``)."""
    for ax in axs:
        ax.grid()
        ax.grid(which="minor", color="0.9")


@gallery("log_demo_1", figsize=(7, 7/3))
def _(fig):
    fig.set_layout_engine("constrained")
    ax1, ax2, ax3 = fig.subplots(1, 3)
    t = np.arange(0.01, 10.0, 0.01)
    ax1.semilogx(t, np.sin(2 * np.pi * t))
    ax1.set(title='semilogx')
    x = np.arange(4)
    ax2.semilogy(4*x, 10**x, 'o--')
    ax2.set(title='semilogy')
    x = np.array([1, 10, 100, 1000])
    ax3.loglog(x, 5 * x, 'o--')
    ax3.set(title='loglog')
    _log_grids(ax1, ax2, ax3)


@gallery("log_demo_2")
def _(fig):
    ax = fig.subplots()
    ax.bar(["L1 cache", "L2 cache", "L3 cache", "RAM", "SSD"],
           [32, 1_000, 32_000, 16_000_000, 512_000_000])
    ax.set_yscale('log', base=2)
    ax.set_yticks([1, 2**10, 2**20, 2**30], labels=['kB', 'MB', 'GB', 'TB'])
    ax.set_title("Typical memory sizes")
    ax.yaxis.grid()


@gallery("log_demo_3", figsize=(6, 3))
def _(fig):
    x = np.linspace(0.0, 2.0, 10)
    y = 10**x
    yerr = 1.75 + 0.75*y
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(1, 2)
    fig.suptitle("errorbars going negative")
    ax1.set_yscale("log", nonpositive='mask')
    ax1.set_title('nonpositive="mask"')
    ax1.errorbar(x, y, yerr=yerr, fmt='o', capsize=5)
    ax2.set_yscale("log", nonpositive='clip')
    ax2.set_title('nonpositive="clip"')
    ax2.errorbar(x, y, yerr=yerr, fmt='o', capsize=5)


@gallery("logit_demo", figsize=(6.4, 8.5))
def _(fig):
    xmax = 10
    x = np.linspace(-xmax, xmax, 10000)
    cdf_norm = [math.erf(w / np.sqrt(2)) / 2 + 1 / 2 for w in x]
    cdf_laplacian = np.where(x < 0, 1 / 2 * np.exp(x), 1 - 1 / 2 * np.exp(-x))
    cdf_cauchy = np.arctan(x) / np.pi + 1 / 2
    axs = fig.subplots(nrows=3, ncols=2)
    for ax in axs.flat:
        ax.plot(x, cdf_norm, label=r"$\mathcal{N}$")
        ax.plot(x, cdf_laplacian, label=r"$\mathcal{L}$")
        ax.plot(x, cdf_cauchy, label="Cauchy")
        ax.legend()
        ax.grid()
    for row, scale in enumerate(({}, {"one_half": "1/2", "use_overline": True})):
        axs[row, 0].set(title="logit scale")
        axs[row, 0].set_yscale("logit", **scale)
        axs[row, 0].set_ylim(1e-5, 1 - 1e-5)
        axs[row, 1].set(title="logit scale")
        axs[row, 1].set_yscale("logit", **scale)
        axs[row, 1].set_xlim(0, xmax)
        axs[row, 1].set_ylim(0.8, 1 - 5e-3)
    axs[2, 0].set(title="linear scale")
    axs[2, 0].set_ylim(0, 1)
    axs[2, 1].set(title="linear scale")
    axs[2, 1].set_xlim(0, xmax)
    axs[2, 1].set_ylim(0.8, 1)
    fig.tight_layout()


def _bivariate_normal(g, n, mean, cov):
    """``n`` draws of a 2-D normal: standard normals through the Cholesky
    factor of ``cov`` (``multivariate_normal(mean, cov, size=n)``)."""
    (a, b), (_, d) = cov
    l11 = math.sqrt(a)
    l21 = b / l11
    l22 = math.sqrt(d - l21 * l21)
    z = g.normal(2 * n).reshape(n, 2)
    return np.column_stack((mean[0] + l11 * z[:, 0],
                            mean[1] + l21 * z[:, 0] + l22 * z[:, 1]))


@gallery("power_norm")
def _(fig):
    g = rng(SEED)
    data = np.vstack([
        _bivariate_normal(g, 100000, [10, 10], [[3, 2], [2, 3]]),
        _bivariate_normal(g, 1000, [30, 20], [[3, 1], [1, 3]]),
    ])
    gammas = [0.8, 0.5, 0.3]
    axs = fig.subplots(nrows=2, ncols=2)
    axs[0, 0].set_title('Linear normalization')
    axs[0, 0].hist2d(data[:, 0], data[:, 1], bins=100)
    for ax, gamma in zip(axs.flat[1:], gammas):
        ax.set_title(r'Power law $(\gamma=%1.1f)$' % gamma)
        ax.hist2d(data[:, 0], data[:, 1], bins=100, norm=PowerNorm(gamma))
    fig.tight_layout()


@gallery("scales", figsize=(6, 8))
def _(fig):
    x = np.arange(400)
    y = np.linspace(0.002, 1, 400)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(3, 2)
    axs[0, 0].plot(x, y)
    axs[0, 0].set_yscale('linear')
    axs[0, 0].set_title('linear')
    axs[0, 1].plot(x, y)
    axs[0, 1].set_yscale('log')
    axs[0, 1].set_title('log')
    axs[1, 0].plot(x, y - y.mean())
    axs[1, 0].set_yscale('symlog', linthresh=0.02)
    axs[1, 0].set_title('symlog')
    axs[1, 1].plot(x, y)
    axs[1, 1].set_yscale('logit')
    axs[1, 1].set_title('logit')
    axs[2, 0].plot(x, y - y.mean())
    axs[2, 0].set_yscale('asinh', linear_width=0.01)
    axs[2, 0].set_title('asinh')

    def forward(x):
        return x**(1/2)

    def inverse(x):
        return x**2

    axs[2, 1].plot(x, y)
    axs[2, 1].set_yscale('function', functions=(forward, inverse))
    axs[2, 1].set_title('function: $x^{1/2}$')
    axs[2, 1].set_yticks(np.arange(0, 1.2, 0.2))
    for ax in axs.flat:
        ax.grid(True)


@gallery("symlog_demo_1")
def _(fig):
    dt = 0.01
    x = np.arange(-50.0, 50.0, dt)
    y = np.arange(0, 100.0, dt)
    ax0, ax1, ax2 = fig.subplots(nrows=3)
    ax0.plot(x, y)
    ax0.set_xscale('symlog')
    ax0.set_ylabel('symlogx')
    ax0.grid()
    ax0.xaxis.grid(which='minor')
    ax1.plot(y, x)
    ax1.set_yscale('symlog')
    ax1.set_ylabel('symlogy')
    ax2.plot(x, np.sin(x / 3.0))
    ax2.set_xscale('symlog')
    ax2.set_yscale('symlog', linthresh=0.015)
    ax2.grid()
    ax2.set_ylabel('symlog both')
    fig.tight_layout()


def _symlog_region(ax, title, **scale):
    """``format_axes`` of ``symlog_demo`` on a line of ``y`` over ``x``
    with a symlog x scale: minor ticks at 2..9, both grids and the linear
    region shaded."""
    x = np.linspace(-60, 60, 201)
    y = np.linspace(0, 100.0, 201)
    ax.plot(x, y)
    ax.set_xscale('symlog', **scale)
    ax.xaxis.get_minor_locator().set_params(subs=[2, 3, 4, 5, 6, 7, 8, 9])
    ax.grid()
    ax.xaxis.grid(which='minor')
    linthresh = ax.xaxis.get_transform().linthresh
    linscale = ax.xaxis.get_transform().linscale
    ax.axvspan(-linthresh, linthresh, color='0.9')
    ax.set_title(title.format(linthresh=linthresh, linscale=linscale))


@gallery("symlog_demo_2")
def _(fig):
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(nrows=2)
    _symlog_region(ax1, 'Linear region: linthresh={linthresh}', linthresh=1)
    _symlog_region(ax2, 'Linear region: linthresh={linthresh}', linthresh=5)


@gallery("symlog_demo_3")
def _(fig):
    title = 'Linear region: linthresh={linthresh}, linscale={linscale}'
    fig.set_layout_engine("constrained")
    ax1, ax2 = fig.subplots(nrows=2)
    _symlog_region(ax1, title, linthresh=1)
    _symlog_region(ax2, title, linthresh=1, linscale=0.1)


@gallery("symlog_demo_4")
def _(fig):
    _symlog_region(fig.subplots(), "Discontinuous gradient at linear/log transition",
                   linscale=0.05)


# --- color -----------------------------------------------------------------------


@gallery("colorbar_basics", figsize=(13, 3))
def _(fig):
    N = 37
    x, y = np.mgrid[:N, :N]
    Z = (np.cos(x*0.2) + np.sin(y*0.3))
    Zpos = np.ma.masked_less(Z, 0)
    Zneg = np.ma.masked_greater(Z, 0)
    ax1, ax2, ax3 = fig.subplots(ncols=3)
    pos = ax1.imshow(Zpos, cmap='Blues', interpolation='none')
    fig.colorbar(pos, ax=ax1)
    neg = ax2.imshow(Zneg, cmap='Reds_r', interpolation='none')
    fig.colorbar(neg, ax=ax2, location='right', anchor=(0, 0.3), shrink=0.7)
    pos_neg_clipped = ax3.imshow(Z, cmap='RdBu', vmin=-1.2, vmax=1.2, interpolation='none')
    cbar = fig.colorbar(pos_neg_clipped, ax=ax3, extend='both')
    cbar.minorticks_on()


@gallery("colorbar_histogram")
def _(fig):
    delta = 0.025
    x = y = np.arange(-2.0, 2.0, delta)
    X, Y = np.meshgrid(x, y)
    Z1 = np.exp(-(((X + 1) * 1.3) ** 2) - ((Y + 1) * 1.3) ** 2)
    Z2 = 2.5 * np.exp(-((X - 1) ** 2) - (Y - 1) ** 2)
    Z = Z1**0.25 - Z2**0.5
    bins = 30
    cmap = mpl.colormaps["RdYlBu_r"]
    bin_edges = np.linspace(Z.min(), Z.max(), bins + 1)
    norm = BoundaryNorm(bin_edges, cmap.N)
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.imshow(Z, cmap=cmap, origin="lower", extent=[-3, 3, -3, 3], norm=norm)
    cax = ax.inset_axes([1.18, 0.02, 0.25, 0.95])
    counts, _ = np.histogram(Z, bins=bin_edges)
    midpoints = (bin_edges[:-1] + bin_edges[1:]) / 2
    distance = midpoints[1] - midpoints[0]
    cax.barh(midpoints, counts, height=0.8 * distance, color=cmap(norm(midpoints)))
    cax.spines[:].set_visible(False)
    cax.set_yticks(bin_edges)
    cax.tick_params(axis="both", which="both", length=0)


@gallery("color_by_yvalue")
def _(fig):
    t = np.arange(0.0, 2.0, 0.01)
    s = np.sin(2 * np.pi * t)
    upper = 0.77
    lower = -0.77
    supper = np.ma.masked_where(s < upper, s)
    slower = np.ma.masked_where(s > lower, s)
    smiddle = np.ma.masked_where((s < lower) | (s > upper), s)
    ax = fig.subplots()
    ax.plot(t, smiddle, t, slower, t, supper)


@gallery("color_cycle_default")
def _(fig):
    def f(x, a):
        """A nice sigmoid-like parametrized curve, ending approximately at *a*."""
        return 0.85 * a * (1 / (1 + np.exp(-x)) + 0.2)

    ax = fig.subplots()
    ax.axis('off')
    ax.set_title("Colors in the default property cycle")
    colors = mpl.rcParams['axes.prop_cycle'].by_key()['color']
    x = np.linspace(-4, 4, 200)
    for i, (color, color_name) in enumerate(zip(colors, TABLEAU_COLORS)):
        assert same_color(color, color_name)
        pos = 4.5 - i
        ax.plot(x, f(x, pos))
        ax.text(4.2, pos, f"'C{i}': '{color_name}'", color=color, va="center")
        ax.bar(9, 1, width=1.5, bottom=pos-0.5)


@gallery("color_demo")
def _(fig):
    t = np.linspace(0.0, 2.0, 201)
    s = np.sin(2 * np.pi * t)
    fig.set_facecolor((.18, .31, .31))
    ax = fig.subplots()
    ax.set_facecolor('#eafff5')
    ax.set_title('Voltage vs. time chart', color='0.7')
    ax.set_xlabel('Time [s]', color='c')
    ax.set_ylabel('Voltage [mV]', color='peachpuff')
    ax.plot(t, s, 'xkcd:crimson')
    ax.plot(t, .7*s, color='C4', linestyle='--')
    ax.tick_params(labelcolor='tab:orange')


COLORMAPS = [
    ('Perceptually Uniform Sequential', [
        'viridis', 'plasma', 'inferno', 'magma', 'cividis']),
    ('Sequential', [
        'Greys', 'Purples', 'Blues', 'Greens', 'Oranges', 'Reds',
        'YlOrBr', 'YlOrRd', 'OrRd', 'PuRd', 'RdPu', 'BuPu',
        'GnBu', 'PuBu', 'YlGnBu', 'PuBuGn', 'BuGn', 'YlGn']),
    ('Sequential (2)', [
        'gray', 'bone', 'pink', 'spring', 'summer', 'autumn', 'winter',
        'cool', 'Wistia', 'hot', 'afmhot', 'gist_heat', 'copper']),
    ('Diverging', [
        'PiYG', 'PRGn', 'BrBG', 'PuOr', 'RdGy', 'RdBu',
        'RdYlBu', 'RdYlGn', 'Spectral', 'coolwarm', 'bwr', 'seismic',
        'berlin', 'managua', 'vanimo']),
    ('Cyclic', ['twilight', 'twilight_shifted', 'hsv']),
    ('Qualitative', [
        'Pastel1', 'Pastel2', 'Paired', 'Accent', 'okabe_ito',
        'Dark2', 'Set1', 'Set2', 'Set3',
        'tab10', 'tab20', 'tab20b', 'tab20c']),
    ('Miscellaneous', [
        'flag', 'prism', 'ocean', 'gist_earth', 'terrain', 'gist_stern',
        'gnuplot', 'gnuplot2', 'CMRmap', 'cubehelix', 'brg',
        'gist_rainbow', 'rainbow', 'jet', 'turbo', 'nipy_spectral',
        'gist_ncar']),
    ("Original and reversed ", ['viridis', 'viridis_r']),
]


def _gradients_height(nrows):
    """The figure height of ``plot_color_gradients`` for ``nrows`` maps."""
    return 0.35 + 0.15 + (nrows + (nrows-1)*0.1)*0.22


def _register_color_gradients(number, cmap_category, cmap_list):
    """Registers ``colormap_reference_<number>``: ``plot_color_gradients``
    of one category of colormaps."""
    figh = _gradients_height(len(cmap_list))

    @gallery(f"colormap_reference_{number}", figsize=(6.4, figh))
    def _(fig):
        gradient = np.linspace(0, 1, 256)
        gradient = np.vstack((gradient, gradient))
        axs = fig.subplots(nrows=len(cmap_list))
        fig.subplots_adjust(top=1-.35/figh, bottom=.15/figh, left=0.2, right=0.99)
        axs[0].set_title(f"{cmap_category} colormaps", fontsize=14)
        for ax, cmap_name in zip(axs, cmap_list):
            ax.imshow(gradient, aspect='auto', cmap=cmap_name)
            ax.text(-.01, .5, cmap_name, va='center', ha='right', fontsize=10,
                    transform=ax.transAxes)
        for ax in axs:
            ax.set_axis_off()


for _number, (_category, _names) in enumerate(COLORMAPS, start=1):
    _register_color_gradients(_number, _category, _names)


@gallery("color_sequences", figsize=(6.4, 9.6))
def _(fig):
    names = [
        'tab10', 'tab20', 'tab20b', 'tab20c', 'Pastel1', 'Pastel2', 'Paired',
        'Accent', 'okabe_ito', 'Dark2', 'Set1', 'Set2', 'Set3', 'petroff6',
        'petroff8', 'petroff10']
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    for n, name in enumerate(names):
        colors = mpl.color_sequences[name]
        x = np.arange(len(colors))
        y = np.full_like(x, n)
        ax.scatter(x, y, facecolor=colors, edgecolor='dimgray', s=200, zorder=2)
    ax.set_yticks(range(len(names)), labels=names)
    ax.grid(visible=True, axis='y')
    ax.yaxis.set_inverted(True)
    ax.xaxis.set_visible(False)
    ax.spines[:].set_visible(False)
    ax.tick_params(left=False)
    ax.set_title('Built In Color Sequences')


def _custom_cmap_grid(fig):
    """The 2 x 2 axes of ``custom_cmap`` and the data they show."""
    x = np.arange(0, np.pi, 0.1)
    y = np.arange(0, 2 * np.pi, 0.1)
    X, Y = np.meshgrid(x, y)
    Z = np.cos(X) * np.sin(Y) * 10
    axs = fig.subplots(2, 2)
    fig.subplots_adjust(left=0.02, bottom=0.06, right=0.95, top=0.94, wspace=0.05)
    return axs, Z


@gallery("custom_cmap_1", figsize=(6, 9))
def _(fig):
    axs, Z = _custom_cmap_grid(fig)
    colors = [(1, 0, 0), (0, 1, 0), (0, 0, 1)]
    for n_bin, ax in zip([3, 6, 10, 100], axs.flat):
        cmap = LinearSegmentedColormap.from_list('my_list', colors, N=n_bin)
        im = ax.imshow(Z, origin='lower', cmap=cmap)
        ax.set_title("N bins: %s" % n_bin)
        fig.colorbar(im, ax=ax)


CDICT1 = {
    'red': ((0.0, 0.0, 0.0), (0.5, 0.0, 0.1), (1.0, 1.0, 1.0)),
    'green': ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0)),
    'blue': ((0.0, 0.0, 1.0), (0.5, 0.1, 0.0), (1.0, 0.0, 0.0)),
}
CDICT2 = {
    'red': ((0.0, 0.0, 0.0), (0.5, 0.0, 1.0), (1.0, 0.1, 1.0)),
    'green': ((0.0, 0.0, 0.0), (1.0, 0.0, 0.0)),
    'blue': ((0.0, 0.0, 0.1), (0.5, 1.0, 0.0), (1.0, 0.0, 0.0)),
}
CDICT3 = {
    'red': ((0.0, 0.0, 0.0), (0.25, 0.0, 0.0), (0.5, 0.8, 1.0), (0.75, 1.0, 1.0),
            (1.0, 0.4, 1.0)),
    'green': ((0.0, 0.0, 0.0), (0.25, 0.0, 0.0), (0.5, 0.9, 0.9), (0.75, 0.0, 0.0),
              (1.0, 0.0, 0.0)),
    'blue': ((0.0, 0.0, 0.4), (0.25, 1.0, 1.0), (0.5, 1.0, 0.8), (0.75, 0.0, 0.0),
             (1.0, 0.0, 0.0)),
}
CDICT4 = {**CDICT3, 'alpha': ((0.0, 1.0, 1.0), (0.5, 0.3, 0.3), (1.0, 1.0, 1.0))}


@gallery("custom_cmap_2", figsize=(6, 9))
def _(fig):
    blue_red1 = LinearSegmentedColormap('BlueRed1', CDICT1)
    for name, cdict in (('BlueRed2', CDICT2), ('BlueRed3', CDICT3), ('BlueRedAlpha', CDICT4)):
        if name not in mpl.colormaps:
            mpl.colormaps.register(LinearSegmentedColormap(name, cdict))
    axs, Z = _custom_cmap_grid(fig)
    im1 = axs[0, 0].imshow(Z, cmap=blue_red1)
    fig.colorbar(im1, ax=axs[0, 0])
    im2 = axs[1, 0].imshow(Z, cmap='BlueRed2')
    fig.colorbar(im2, ax=axs[1, 0])
    mpl.rcParams['image.cmap'] = 'BlueRed3'
    im3 = axs[0, 1].imshow(Z)
    fig.colorbar(im3, ax=axs[0, 1])
    axs[0, 1].set_title("Alpha = 1")
    axs[1, 1].plot([0, 10 * np.pi], [0, 20 * np.pi], color='c', lw=20, zorder=-1)
    im4 = axs[1, 1].imshow(Z)
    fig.colorbar(im4, ax=axs[1, 1])
    im4.set_cmap('BlueRedAlpha')
    axs[1, 1].set_title("Varying alpha")
    fig.suptitle('Custom Blue-Red colormaps', fontsize=16)
    fig.subplots_adjust(top=0.9)


def _fan(fig, colors):
    """``individual_colors_from_cmap``: one line from 0 to ``i`` per colour."""
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    for i, color in enumerate(colors):
        ax.plot([0, i], color=color)


@gallery("individual_colors_from_cmap_1")
def _(fig):
    _fan(fig, mpl.colormaps['plasma'](np.linspace(0, 1, 21)))


@gallery("individual_colors_from_cmap_2")
def _(fig):
    _fan(fig, mpl.colormaps['Dark2'].colors)


#: ``plot_colortable``'s layout in pixels at 72 dpi.
CELL_WIDTH, CELL_HEIGHT, SWATCH_WIDTH, MARGIN = 212, 22, 48, 12


def _colortable_size(n, ncols):
    """Rows and size in pixels of a colour table of ``n`` colours."""
    nrows = math.ceil(n / ncols)
    return nrows, CELL_WIDTH * ncols + 2 * MARGIN, CELL_HEIGHT * nrows + 2 * MARGIN


def _register_colortable(number, colors, ncols, sort_colors):
    """Registers ``named_colors_<number>``: ``plot_colortable`` of one table
    of named colours."""
    nrows, width, height = _colortable_size(len(colors), ncols)

    @gallery(f"named_colors_{number}", figsize=(width / 72, height / 72), dpi=72)
    def _(fig):
        if sort_colors:
            names = sorted(colors, key=lambda c: tuple(mpl.colors.rgb_to_hsv(mpl.colors.to_rgb(c))))
        else:
            names = list(colors)
        ax = fig.subplots()
        fig.subplots_adjust(MARGIN/width, MARGIN/height,
                            (width-MARGIN)/width, (height-MARGIN)/height)
        ax.set_xlim(0, CELL_WIDTH * ncols)
        ax.set_ylim(CELL_HEIGHT * (nrows-0.5), -CELL_HEIGHT/2.)
        ax.yaxis.set_visible(False)
        ax.xaxis.set_visible(False)
        ax.set_axis_off()
        for i, name in enumerate(names):
            row = i % nrows
            col = i // nrows
            y = row * CELL_HEIGHT
            swatch_start_x = CELL_WIDTH * col
            text_pos_x = CELL_WIDTH * col + SWATCH_WIDTH + 7
            ax.text(text_pos_x, y, name, fontsize=14,
                    horizontalalignment='left', verticalalignment='center')
            ax.add_patch(Rectangle(xy=(swatch_start_x, y-9), width=SWATCH_WIDTH,
                                   height=18, facecolor=colors[name], edgecolor='0.7'))


_register_colortable(1, mpl.colors.BASE_COLORS, ncols=3, sort_colors=False)
_register_colortable(2, mpl.colors.TABLEAU_COLORS, ncols=2, sort_colors=False)
_register_colortable(3, mpl.colors.CSS4_COLORS, ncols=4, sort_colors=True)


@gallery("set_alpha", figsize=(8, 4))
def _(fig):
    ax1, ax2 = fig.subplots(ncols=2)
    x_values = [n for n in range(20)]
    y_values = rng(SEED).normal(20)
    facecolors = ['green' if y > 0 else 'red' for y in y_values]
    edgecolors = facecolors
    ax1.bar(x_values, y_values, color=facecolors, edgecolor=edgecolors, alpha=0.5)
    ax1.set_title("Explicit 'alpha' keyword value\nshared by all bars and edges")
    abs_y = [abs(y) for y in y_values]
    face_alphas = [n / max(abs_y) for n in abs_y]
    edge_alphas = [1 - alpha for alpha in face_alphas]
    colors_with_alphas = list(zip(facecolors, face_alphas))
    edgecolors_with_alphas = list(zip(edgecolors, edge_alphas))
    ax2.bar(x_values, y_values, color=colors_with_alphas, edgecolor=edgecolors_with_alphas)
    ax2.set_title('Normalized alphas for\neach bar and each edge')


# --- units -----------------------------------------------------------------------


@gallery("annotate_with_units")
def _(fig):
    ax = fig.subplots()
    ax.annotate("Note 01", [0.5*cm, 0.5*cm])
    ax.annotate('local max', xy=(3*cm, 1*cm), xycoords='data',
                xytext=(0.8*cm, 0.95*cm), textcoords='data',
                arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='right', verticalalignment='top')
    ax.annotate('local max', xy=(3*cm, 1*cm), xycoords='data',
                xytext=(0.8, 0.95), textcoords='axes fraction',
                arrowprops=dict(facecolor='black', shrink=0.05),
                horizontalalignment='right', verticalalignment='top')
    ax.set_xlim(0*cm, 4*cm)
    ax.set_ylim(0*cm, 4*cm)


@gallery("artist_tests")
def _(fig):
    ax = fig.subplots()
    ax.xaxis.set_units(cm)
    ax.yaxis.set_units(cm)
    line = mlines.Line2D([0*cm, 1.5*cm], [0*cm, 2.5*cm], lw=2, color='black', axes=ax)
    ax.add_line(line)
    t = mtext.Text(3*cm, 2.5*cm, 'text label', ha='left', va='bottom', axes=ax)
    ax.add_artist(t)
    ax.set_xlim(-1*cm, 10*cm)
    ax.set_ylim(-1*cm, 10*cm)
    ax.grid(True)
    ax.set_title("Artists with units")


@gallery("bar_demo2")
def _(fig):
    cms = cm * np.arange(0, 10, 2)
    bottom = 0 * cm
    width = 0.8 * cm
    axs = fig.subplots(2, 2)
    axs[0, 0].bar(cms, cms, bottom=bottom)
    axs[0, 1].bar(cms, cms, bottom=bottom, width=width, xunits=cm, yunits=inch)
    axs[1, 0].bar(cms, cms, bottom=bottom, width=width, xunits=inch, yunits=cm)
    axs[1, 0].set_xlim(2, 6)
    axs[1, 1].bar(cms, cms, bottom=bottom, width=width, xunits=inch, yunits=inch)
    axs[1, 1].set_xlim(2 * cm, 6 * cm)
    fig.tight_layout()


@gallery("bar_unit_demo")
def _(fig):
    N = 5
    tea_means = [15*cm, 10*cm, 8*cm, 12*cm, 5*cm]
    tea_std = [2*cm, 1*cm, 1*cm, 4*cm, 2*cm]
    ax = fig.subplots()
    ax.yaxis.set_units(inch)
    ind = np.arange(N)
    width = 0.35
    ax.bar(ind, tea_means, width, bottom=0*cm, yerr=tea_std, label='Tea')
    coffee_means = (14*cm, 19*cm, 7*cm, 5*cm, 10*cm)
    coffee_std = (3*cm, 5*cm, 2*cm, 1*cm, 2*cm)
    ax.bar(ind + width, coffee_means, width, bottom=0*cm, yerr=coffee_std, label='Coffee')
    ax.set_title('Cup height by group and beverage choice')
    ax.set_xticks(ind + width / 2, labels=['G1', 'G2', 'G3', 'G4', 'G5'])
    ax.legend()
    ax.autoscale_view()


def _ellipse_compare(fig, outline):
    """A figure of ``ellipse_with_units``: an ellipse drawn by ``outline``
    (``Ellipse`` or ``Arc``) over a filled polygon approximating it, with
    automatic and equal aspect."""
    xcenter, ycenter = 0.38*cm, 0.52*cm
    width, height = 1e-1*cm, 3e-1*cm
    angle = -30
    theta = np.deg2rad(np.arange(0.0, 360.0, 1.0))
    x = 0.5 * width * np.cos(theta)
    y = 0.5 * height * np.sin(theta)
    rtheta = np.radians(angle)
    R = np.array([
        [np.cos(rtheta), -np.sin(rtheta)],
        [np.sin(rtheta), np.cos(rtheta)],
    ])
    x, y = np.dot(R, [x, y])
    x += xcenter
    y += ycenter
    ax = fig.add_subplot(211, aspect='auto')
    ax.fill(x, y, alpha=0.2, facecolor='yellow', edgecolor='yellow', linewidth=1, zorder=1)
    ax.add_patch(outline((xcenter, ycenter), width, height, angle=angle, linewidth=2,
                         fill=False, zorder=2))
    ax = fig.add_subplot(212, aspect='equal')
    ax.fill(x, y, alpha=0.2, facecolor='green', edgecolor='green', zorder=1)
    ax.add_patch(outline((xcenter, ycenter), width, height, angle=angle, linewidth=2,
                         fill=False, zorder=2))


@gallery("ellipse_with_units_1")
def _(fig):
    _ellipse_compare(fig, mpatches.Ellipse)


@gallery("ellipse_with_units_2")
def _(fig):
    _ellipse_compare(fig, mpatches.Arc)


class Foo:
    """``evans_test``'s value carrying a scalar conversion factor as its
    unit."""

    def __init__(self, val, unit=1.0):
        self.unit = unit
        self._val = val * unit

    def value(self, unit):
        if unit is None:
            unit = self.unit
        return self._val / unit


class FooConverter(munits.ConversionInterface):
    """``evans_test``'s converter of :class:`Foo` values."""

    @staticmethod
    def axisinfo(unit, axis):
        if unit == 1.0 or unit == 2.0:
            return munits.AxisInfo(
                majloc=ticker.IndexLocator(8, 0),
                majfmt=ticker.FormatStrFormatter("VAL: %s"),
                label='foo',
            )
        else:
            return None

    @staticmethod
    def convert(obj, unit, axis):
        if np.iterable(obj):
            return [o.value(unit) for o in obj]
        else:
            return obj.value(unit)

    @staticmethod
    def default_units(x, axis):
        if np.iterable(x):
            for thisx in x:
                return thisx.unit
        else:
            return x.unit


munits.registry[Foo] = FooConverter()


@gallery("evans_test")
def _(fig):
    x = [Foo(val, 1.0) for val in range(0, 50, 2)]
    y = [i for i in range(len(x))]
    ax1, ax2 = fig.subplots(1, 2)
    fig.suptitle("Custom units")
    fig.subplots_adjust(bottom=0.2)
    ax2.plot(x, y, 'o', xunits=2.0)
    ax2.set_title("xunits = 2.0")
    ax2.tick_params(axis='x', rotation=30, rotation_mode='xtick')
    ax1.plot(x, y)
    ax1.set_title('default units')
    ax1.tick_params(axis='x', rotation=30, rotation_mode='xtick')


@gallery("radian_demo")
def _(fig):
    x = [val*radians for val in np.arange(0, 15, 0.01)]
    axs = fig.subplots(2)
    axs[0].plot(x, basic_units.cos(x), xunits=radians)
    axs[1].plot(x, basic_units.cos(x), xunits=degrees)
    fig.tight_layout()


@gallery("units_sample")
def _(fig):
    cms = cm * np.arange(0, 10, 2)
    fig.set_layout_engine("constrained")
    axs = fig.subplots(2, 2)
    axs[0, 0].plot(cms, cms)
    axs[0, 1].plot(cms, cms, xunits=cm, yunits=inch)
    axs[1, 0].plot(cms, cms, xunits=inch, yunits=cm)
    axs[1, 0].set_xlim(-1, 4)
    axs[1, 1].plot(cms, cms, xunits=inch, yunits=inch)
    axs[1, 1].set_xlim(3*cm, 6*cm)


@gallery("units_scatter")
def _(fig):
    data = (1, 2, 3, 4, 5, 6, 7, 8)
    mask = (1, 0, 1, 0, 0, 0, 1, 0)
    xsecs = secs * np.ma.MaskedArray(data, mask, float)
    ax1, ax2, ax3 = fig.subplots(nrows=3, sharex=True)
    ax1.scatter(xsecs, xsecs)
    ax1.yaxis.set_units(secs)
    ax2.scatter(xsecs, xsecs, yunits=hertz)
    ax3.scatter(xsecs, xsecs, yunits=minutes)
    fig.tight_layout()
