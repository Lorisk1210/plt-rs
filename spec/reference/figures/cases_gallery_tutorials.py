"""The gallery's tutorials (``galleries/tutorials``: ``artists``,
``coding_shortcuts``, ``images``, ``lifecycle`` and ``pyplot``, one case per
figure, ``<script>_<n>``), the first frame of its animations
(``galleries/examples/animation`` and the ``animations`` page of
``galleries/users_explain/animations``) and the figures its event handling
examples (``galleries/examples/event_handling``) show before any
interaction.

The scripts run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng`` (a fresh generator per figure), and the
stinkbug photograph of the image tutorial from a synthetic grey picture of
its size (its thumbnail from the same picture drawn at 64 x 48). pyplot
calls act on the case's figure and its axes; ``plt.show()``, callbacks,
pickers and ``draggable`` calls (which only matter interactively) are
dropped.

An animation's first frame is the figure as it is before the animation
starts, or what its ``init_func`` draws; an ``ArtistAnimation`` shows the
artists of its first frame only (it hides the others, and leaves out its
``animated`` images), and an animation drawn by a loop (``animation_demo``,
``frame_grabbing_sgskip``, which writes its frames to a movie through
ffmpeg) shows the frame of its first pass. Event handling examples that simulate an event
themselves (``cursor_demo``'s mouse move to the data point (0.5, 0.5)) show
its outcome. The editors' artists that blitting callbacks draw over every
redraw (``path_editor``, ``poly_editor``) are made ordinary artists, which
draws them the same way in every output format; ``path_editor_blitted`` and
``poly_editor_blitted`` run the editors as written instead (their animated
artists drawn by their draw callbacks, then a vertex dragged with simulated
mouse events) and are taken from the canvas.

Covered elsewhere: the fifth and ninth ``pyplot`` figures
(``gallery_misc/keyword_plotting`` and ``gallery_text/annotation_basic``),
the ``pyplots`` examples (``gallery_misc/pyplot_simple`` and
``gallery_misc/pyplot_two_subplots``) and the ``spines`` examples
(``gallery_misc/spine_placement_demo``, ``gallery_misc/spines``,
``spines/dropped`` and ``spines/centered_with_arrows``).

Not ported: ``rain`` (blank before the animation starts: every drop has
size 0 and a transparent colour),
``ginput_manual_clabel_sgskip`` and ``pong_sgskip`` (interactive only), and
the ``blitting`` page (its figures hold animated artists only).
"""

import matplotlib as mpl
import numpy as np
from matplotlib import colors as mcolors
from matplotlib.backend_bases import MouseButton, MouseEvent
from matplotlib.collections import RegularPolyCollection
from matplotlib.lines import Line2D
from matplotlib.patches import Circle, ConnectionPatch, PathPatch, Polygon, Rectangle
from matplotlib.path import Path
from matplotlib.tri import Triangulation

from spec_support import case, rng

G = "gallery_tutorials"
SEED = 19680801


def tutorial(name, **options):
    """Registers a case of this group."""
    return case(f"{G}/{name}", **options)


def _rows(g, rows, cols):
    """``np.random.rand(rows, cols)``."""
    return g.uniform(rows * cols).reshape(rows, cols)


# === tutorials/artists ========================================================


@tutorial("artists_1")
def _(fig):
    fig.subplots_adjust(top=0.8)
    ax1 = fig.add_subplot(211)
    ax1.set_ylabel('Voltage [V]')
    ax1.set_title('A sine wave')
    t = np.arange(0.0, 1.0, 0.01)
    ax1.plot(t, np.sin(2*np.pi*t), color='blue', lw=2)
    ax2 = fig.add_axes((0.15, 0.1, 0.7, 0.3))
    ax2.hist(rng(SEED).normal(1000), 50, facecolor='yellow', edgecolor='yellow')
    ax2.set_xlabel('Time [s]')


@tutorial("artists_2")
def _(fig):
    l1 = Line2D([0, 1], [0, 1], transform=fig.transFigure, figure=fig)
    l2 = Line2D([0, 1], [1, 0], transform=fig.transFigure, figure=fig)
    fig.lines.extend([l1, l2])


# === tutorials/coding_shortcuts ===============================================


def _daylight():
    """Daylight hours in London over the days of a year."""
    day = np.arange(365)
    return day, 4.276 * np.sin(2 * np.pi * (day - 80)/365) + 12.203


@tutorial("coding_shortcuts_1")
def _(fig):
    day, hours = _daylight()
    ax = fig.subplots()
    ax.plot(day, hours, color="orange")
    ax.set_xlabel("day")
    ax.set_ylabel("daylight hours")
    ax.set_title("London")


@tutorial("coding_shortcuts_2")
def _(fig):
    day, hours = _daylight()
    ax = fig.subplots()
    ax.plot(day, hours, color="orange")
    ax.set(xlabel="day", ylabel="daylight hours", title="London")


@tutorial("coding_shortcuts_3")
def _(fig):
    day, hours = _daylight()
    ax = fig.add_subplot()
    ax.plot(day, hours, color="orange")
    ax.set(xlabel="day", ylabel="daylight hours", title="London")


# ``plt.axes(**kwargs)`` (the fifth figure) is ``fig.add_subplot(**kwargs)``.
@tutorial("coding_shortcuts_4")
@tutorial("coding_shortcuts_5")
def _(fig):
    day, hours = _daylight()
    ax = fig.add_subplot(xlabel="day", ylabel="daylight hours", title="London")
    ax.plot(day, hours, color="orange")


@tutorial("coding_shortcuts_6")
def _(fig):
    day, hours = _daylight()
    fig.add_subplot(xlabel="day", ylabel="daylight hours").plot(day, hours, color="orange")


# === tutorials/images =========================================================


def _stinkbug(width=500, height=375):
    """A grey RGB uint8 picture in place of ``stinkbug.png``: a dark oval bug
    with rings on its back, on a ground lit from the left, with a fine
    diagonal grain; the red channel is a little brighter and the blue one a
    little darker, as in the photograph. Smaller sizes draw the same picture
    (``img.thumbnail``)."""
    v, u = np.mgrid[0:height, 0:width]
    u = (u + 0.5) / width
    v = (v + 0.5) / height
    grain = 12 * ((37 * u + 23 * v) % 1)
    ground = 170 - 60 * u + 30 * v + grain
    body = ((u - 0.5) / 0.3) ** 2 + ((v - 0.55) / 0.22) ** 2
    bug = 25 + 60 * body + 30 * ((6 * body) % 1) + grain
    grey = np.where(body < 1, bug, ground)
    return np.clip(np.dstack([grey + 6, grey, grey - 8]), 0, 255).astype(np.uint8)


def _luminosity():
    """The red channel of the picture, a pseudocolor image."""
    return _stinkbug()[:, :, 0]


@tutorial("images_1")
def _(fig):
    fig.subplots().imshow(_stinkbug())


@tutorial("images_2")
def _(fig):
    fig.subplots().imshow(_luminosity())


@tutorial("images_3")
def _(fig):
    fig.subplots().imshow(_luminosity(), cmap="hot")


@tutorial("images_4")
def _(fig):
    imgplot = fig.subplots().imshow(_luminosity())
    imgplot.set_cmap('nipy_spectral')


@tutorial("images_5")
def _(fig):
    imgplot = fig.subplots().imshow(_luminosity())
    fig.colorbar(imgplot)


@tutorial("images_6")
def _(fig):
    fig.subplots().hist(_luminosity().ravel(), bins=range(256), fc='k', ec='k')


@tutorial("images_7")
def _(fig):
    fig.subplots().imshow(_luminosity(), clim=(0, 175))


@tutorial("images_8")
def _(fig):
    imgplot = fig.subplots().imshow(_luminosity())
    imgplot.set_clim(0, 175)


@tutorial("images_9")
def _(fig):
    fig.subplots().imshow(_stinkbug(64, 48))


@tutorial("images_10")
def _(fig):
    fig.subplots().imshow(_stinkbug(64, 48), interpolation="bilinear")


@tutorial("images_11")
def _(fig):
    fig.subplots().imshow(_stinkbug(64, 48), interpolation="bicubic")


# === tutorials/lifecycle ======================================================

_REVENUE = {
    'Barton LLC': 109438.50,
    'Frami, Hills and Schmidt': 103569.59,
    'Fritsch, Russel and Anderson': 112214.71,
    'Jerde-Hilpert': 112591.43,
    'Keeling LLC': 100934.30,
    'Koepp Ltd': 103660.54,
    'Kulas Inc': 137351.96,
    'Trantow-Barrows': 123381.38,
    'White-Trantow': 135841.99,
    'Will LLC': 104437.60,
}
_FIVETHIRTYEIGHT = dict(mpl.style.library['fivethirtyeight'])
_AUTOLAYOUT = {**_FIVETHIRTYEIGHT, 'figure.autolayout': True}


def _revenue(fig):
    """The companies' revenue as horizontal bars."""
    ax = fig.subplots()
    ax.barh(list(_REVENUE.keys()), list(_REVENUE.values()))
    return ax


def _slanted_revenue(fig):
    """:func:`_revenue` with its x tick labels slanted."""
    ax = _revenue(fig)
    labels = ax.get_xticklabels()
    mpl.artist.setp(labels, rotation=45, horizontalalignment='right')
    return ax


def _titled_revenue(fig):
    """:func:`_slanted_revenue` with limits, labels and a title."""
    ax = _slanted_revenue(fig)
    ax.set(xlim=(-10000, 140000), xlabel='Total Revenue', ylabel='Company',
           title='Company Revenue')
    return ax


def _currency(x, pos):
    """The two arguments are the value and tick position"""
    if x >= 1e6:
        s = f'${x*1e-6:1.1f}M'
    else:
        s = f'${x*1e-3:1.0f}K'
    return s


@tutorial("lifecycle_1")
def _(fig):
    fig.subplots()


@tutorial("lifecycle_2")
def _(fig):
    _revenue(fig)


@tutorial("lifecycle_3", rc=_FIVETHIRTYEIGHT)
def _(fig):
    _revenue(fig)


@tutorial("lifecycle_4", rc=_FIVETHIRTYEIGHT)
def _(fig):
    _revenue(fig).get_xticklabels()


@tutorial("lifecycle_5", rc=_FIVETHIRTYEIGHT)
def _(fig):
    _slanted_revenue(fig)


@tutorial("lifecycle_6", rc=_AUTOLAYOUT)
def _(fig):
    _slanted_revenue(fig)


@tutorial("lifecycle_7", rc=_AUTOLAYOUT)
def _(fig):
    _titled_revenue(fig)


@tutorial("lifecycle_8", figsize=(8, 4), rc=_AUTOLAYOUT)
def _(fig):
    _titled_revenue(fig)


@tutorial("lifecycle_9", figsize=(6, 8), rc=_AUTOLAYOUT)
def _(fig):
    _titled_revenue(fig).xaxis.set_major_formatter(_currency)


@tutorial("lifecycle_10", figsize=(8, 8), rc=_AUTOLAYOUT)
def _(fig):
    ax = _slanted_revenue(fig)
    ax.axvline(np.mean(list(_REVENUE.values())), ls='--', color='r')
    for group in [3, 5, 8]:
        ax.text(145000, group, "New Company", fontsize=10,
                verticalalignment="center")
    ax.title.set(y=1.05)
    ax.set(xlim=(-10000, 140000), xlabel='Total Revenue', ylabel='Company',
           title='Company Revenue')
    ax.xaxis.set_major_formatter(_currency)
    ax.set_xticks([0, 25e3, 50e3, 75e3, 100e3, 125e3])
    fig.subplots_adjust(right=.1)


# === tutorials/pyplot =========================================================


@tutorial("pyplot_1")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3, 4])
    ax.set_ylabel('some numbers')


@tutorial("pyplot_2")
def _(fig):
    fig.subplots().plot([1, 2, 3, 4], [1, 4, 9, 16])


@tutorial("pyplot_3")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3, 4], [1, 4, 9, 16], 'ro')
    ax.axis((0, 6, 0, 20))


@tutorial("pyplot_4")
def _(fig):
    t = np.arange(0., 5., 0.2)
    fig.subplots().plot(t, t, 'r--', t, t**2, 'bs', t, t**3, 'g^')


@tutorial("pyplot_6", figsize=(9, 3))
def _(fig):
    names = ['group_a', 'group_b', 'group_c']
    values = [1, 10, 100]
    fig.add_subplot(131).bar(names, values)
    fig.add_subplot(132).scatter(names, values)
    fig.add_subplot(133).plot(names, values)
    fig.suptitle('Categorical Plotting')


@tutorial("pyplot_7")
def _(fig):
    def f(t):
        return np.exp(-t) * np.cos(2*np.pi*t)

    t1 = np.arange(0.0, 5.0, 0.1)
    t2 = np.arange(0.0, 5.0, 0.02)
    fig.add_subplot(211).plot(t1, f(t1), 'bo', t2, f(t2), 'k')
    fig.add_subplot(212).plot(t2, np.cos(2*np.pi*t2), 'r--')


@tutorial("pyplot_8")
def _(fig):
    mu, sigma = 100, 15
    x = mu + sigma * rng(SEED).normal(10000)
    ax = fig.subplots()
    ax.hist(x, 50, density=True, facecolor='g', alpha=0.75)
    ax.set_xlabel('Smarts')
    ax.set_ylabel('Probability')
    ax.set_title('Histogram of IQ')
    ax.text(60, .025, r'$\mu=100,\ \sigma=15$')
    ax.axis([40, 160, 0, 0.03])
    ax.grid(True)


@tutorial("pyplot_10")
def _(fig):
    y = rng(SEED).normal(1000, loc=0.5, scale=0.4)
    y = y[(y > 0) & (y < 1)]
    y.sort()
    x = np.arange(len(y))
    for position, (scale, data, options) in enumerate([
            ('linear', y, {}),
            ('log', y, {}),
            ('symlog', y - y.mean(), {'linthresh': 0.01}),
            ('logit', y, {})], start=221):
        ax = fig.add_subplot(position)
        ax.plot(x, data)
        ax.set_yscale(scale, **options)
        ax.set_title(scale)
        ax.grid(True)
    fig.subplots_adjust(top=0.92, bottom=0.08, left=0.10, right=0.95, hspace=0.25,
                        wspace=0.35)


# === examples/animation =======================================================


@tutorial("animate_decay")
def _(fig):
    ax = fig.subplots()
    line, = ax.plot([], [], lw=2)
    ax.grid()
    ax.set_ylim(-1.1, 1.1)
    ax.set_xlim(0, 1)
    line.set_data([], [])


@tutorial("animated_histogram")
def _(fig):
    ax = fig.subplots()
    ax.hist(rng(SEED).normal(1000), np.linspace(-4, 4, 100), lw=1,
            ec="yellow", fc="green", alpha=0.5)
    ax.set_ylim(top=55)


@tutorial("animation_demo")
def _(fig):
    ax = fig.subplots()
    ax.imshow(_rows(rng(SEED), 50, 50))
    ax.set_title("frame 0")


@tutorial("bayes_update")
def _(fig):
    ax = fig.subplots()
    ax.plot([], [], 'k-')
    ax.set_xlim(0, 1)
    ax.set_ylim(0, 10)
    ax.grid(True)
    ax.axvline(0.7, linestyle='--', color='black')


@tutorial("double_pendulum", figsize=(5, 4))
def _(fig):
    length = 2.0
    ax = fig.add_subplot(autoscale_on=False, xlim=(-length, length), ylim=(-length, 1.))
    ax.set_aspect('equal')
    ax.grid()
    ax.plot([], [], 'o-', lw=2)
    ax.plot([], [], '.-', lw=1, ms=2)
    ax.text(0.05, 0.9, '', transform=ax.transAxes)


@tutorial("frame_grabbing_sgskip")
def _(fig):
    g = rng(SEED)
    x0, y0 = 0, 0
    x0 += 0.1 * g.normal(1)[0]
    y0 += 0.1 * g.normal(1)[0]
    ax = fig.subplots()
    ax.plot([x0], [y0], 'k-o')
    ax.set_xlim(-5, 5)
    ax.set_ylim(-5, 5)


@tutorial("dynamic_image")
def _(fig):
    x = np.linspace(0, 2 * np.pi, 120) + np.pi / 15
    y = np.linspace(0, 2 * np.pi, 100).reshape(-1, 1) + np.pi / 30
    fig.subplots().imshow(np.sin(x) + np.cos(y))


@tutorial("multiple_axes", figsize=(6, 2))
def _(fig):
    axl, axr = fig.subplots(ncols=2, sharey=True,
                            gridspec_kw=dict(width_ratios=[1, 3], wspace=0))
    axl.set_aspect(1)
    axr.set_box_aspect(1 / 3)
    axr.yaxis.set_visible(False)
    axr.xaxis.set_ticks([0, np.pi, 2 * np.pi], ["0", r"$\pi$", r"$2\pi$"])
    x = np.linspace(0, 2 * np.pi, 50)
    axl.plot(np.cos(x), np.sin(x), "k", lw=0.3)
    axl.plot(0, 0, "o")
    axr.plot(x, np.sin(x))
    fig.add_artist(ConnectionPatch((1, 0), (0, 0), "data", "data", axesA=axl, axesB=axr,
                                   color="C0", ls="dotted"))


@tutorial("pause_resume")
def _(fig):
    ax = fig.subplots()
    ax.set_title('Click to pause/resume the animation')
    x = np.linspace(-0.1, 0.1, 1000)
    ax.plot(x, (1.0 / ((4 * np.pi * 2e-4 * 0.1) ** 0.5)
                * np.exp(-x ** 2 / (4 * 2e-4 * 0.1))))


@tutorial("random_walk")
def _(fig):
    ax = fig.add_subplot(projection="3d")
    for _ in range(40):
        ax.plot([], [], [])
    ax.set(xlim3d=(0, 1), xlabel='X')
    ax.set(ylim3d=(0, 1), ylabel='Y')
    ax.set(zlim3d=(0, 1), zlabel='Z')


@tutorial("simple_anim")
def _(fig):
    x = np.arange(0, 2*np.pi, 0.01)
    fig.subplots().plot(x, np.sin(x))


@tutorial("simple_scatter")
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.scatter(1, 0)


@tutorial("strip_chart")
def _(fig):
    ax = fig.subplots()
    ax.add_line(Line2D([0], [0]))
    ax.set_ylim(-.1, 1.1)
    ax.set_xlim(0, 2)


@tutorial("unchained", figsize=(8, 8))
def _(fig):
    fig.set_facecolor('black')
    ax = fig.add_subplot(frameon=False)
    data = _rows(rng(SEED), 64, 75)
    X = np.linspace(-1, 1, data.shape[-1])
    G = 1.5 * np.exp(-4 * X ** 2)
    for i in range(len(data)):
        xscale = 1 - i / 200.
        lw = 1.5 - i / 100.0
        ax.plot(xscale * X, i + G * data[i], color="w", lw=lw)
    ax.set_ylim(-1, 70)
    ax.set_xticks([])
    ax.set_yticks([])
    ax.text(0.5, 1.0, "MATPLOTLIB ", transform=ax.transAxes,
            ha="right", va="bottom", color="w",
            family="sans-serif", fontweight="light", fontsize=16)
    ax.text(0.5, 1.0, "UNCHAINED", transform=ax.transAxes,
            ha="left", va="bottom", color="w",
            family="sans-serif", fontweight="bold", fontsize=16)


# === users_explain/animations =================================================


@tutorial("animations_1")
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 3, 40)
    g = -9.81
    v0 = 12
    z = g * t**2 / 2 + v0 * t
    v02 = 5
    z2 = g * t**2 / 2 + v02 * t
    ax.scatter(t[0], z[0], c="b", s=5, label=f'v0 = {v0} m/s')
    ax.plot(t[0], z2[0], label=f'v0 = {v02} m/s')
    ax.set(xlim=(0, 3), ylim=(-4, 10), xlabel='Time [s]', ylabel='Z [m]')
    ax.legend()


@tutorial("animations_2")
def _(fig):
    ax = fig.subplots()
    g = rng(SEED)
    data = np.array([20, 20, 20, 20])
    x = np.array([1, 2, 3, 4])
    colors = ['tab:blue', 'tab:red', 'tab:green', 'tab:purple']
    for i in range(20):
        data += g.integers(4, 0, 10)
        container = ax.barh(x, data, color=colors)
        if i > 0:
            for bar in container:
                bar.set_visible(False)


# === examples/event_handling ==================================================


@tutorial("close_event")
def _(fig):
    fig.add_subplot().text(0.35, 0.5, 'Close Me!', dict(size=30))


@tutorial("coords_demo")
def _(fig):
    t = np.arange(0.0, 1.0, 0.01)
    fig.subplots().plot(t, np.sin(2 * np.pi * t))


def _cursor(fig, title, snap):
    """``cursor_demo``'s points with the cross hair where its simulated
    mouse move to the data point (0.5, 0.5) puts it: there, or on the
    point with the next x value if it ``snap``\\s."""
    x = np.arange(0, 1, 0.01)
    y = np.sin(2 * 2 * np.pi * x)
    ax = fig.subplots()
    ax.set_title(title)
    ax.plot(x, y, 'o')
    horizontal_line = ax.axhline(color='k', lw=0.8, ls='--')
    vertical_line = ax.axvline(color='k', lw=0.8, ls='--')
    text = ax.text(0.72, 0.9, '', transform=ax.transAxes)
    cx, cy = 0.5, 0.5
    if snap:
        index = min(np.searchsorted(x, cx), len(x) - 1)
        cx, cy = x[index], y[index]
    horizontal_line.set_ydata([cy])
    vertical_line.set_xdata([cx])
    text.set_text(f'x={cx:1.2f}, y={cy:1.2f}')


@tutorial("cursor_demo_1")
def _(fig):
    _cursor(fig, 'Simple cursor', snap=False)


@tutorial("cursor_demo_2")
def _(fig):
    _cursor(fig, 'Blitted cursor', snap=False)


@tutorial("cursor_demo_3")
def _(fig):
    _cursor(fig, 'Snapping cursor', snap=True)


def _means_and_deviations(ax, samples):
    """The mean and standard deviation of 100 series of ``samples`` uniform
    draws, as points on ``ax``."""
    X = _rows(rng(SEED), 100, samples)
    xs = np.mean(X, axis=1)
    ys = np.std(X, axis=1)
    ax.set_title('click on point to plot time series')
    ax.plot(xs, ys, 'o', picker=True, pickradius=5)
    return xs, ys


@tutorial("data_browser")
def _(fig):
    ax, _ = fig.subplots(2, 1)
    xs, ys = _means_and_deviations(ax, 200)
    ax.text(0.05, 0.95, 'selected: none', transform=ax.transAxes, va='top')
    ax.plot([xs[0]], [ys[0]], 'o', ms=12, alpha=0.4, color='yellow', visible=False)


@tutorial("figure_axes_enter_leave")
def _(fig):
    fig.subplots(2, 1)
    fig.suptitle('mouse hover over figure or Axes to trigger events')


@tutorial("image_slices_viewer")
def _(fig):
    x, y, z = np.ogrid[-10:10:100j, -10:10:100j, 1:10:20j]
    X = np.sin(x * y * z) / (x * y * z)
    ax = fig.subplots()
    ax.imshow(X[:, :, 0])
    ax.set_title('Use scroll wheel to navigate\nindex 0')


@tutorial("keypress_demo")
def _(fig):
    g = rng(SEED)
    ax = fig.subplots()
    ax.plot(g.uniform(12), g.uniform(12), 'go')
    ax.set_xlabel('easy come, easy go')
    ax.set_title('Press a key')


@tutorial("lasso_demo")
def _(fig):
    ax = fig.add_subplot(xlim=(0, 1), ylim=(0, 1),
                         title='Lasso points using left mouse button')
    data = _rows(rng(SEED), 100, 2)
    ax.add_collection(RegularPolyCollection(
        6, sizes=(100,), offset_transform=ax.transData,
        offsets=data, array=np.zeros(len(data)),
        clim=(0, 1), cmap=mcolors.ListedColormap(["tab:blue", "tab:red"])))


@tutorial("legend_picking")
def _(fig):
    t = np.linspace(0, 1)
    ax = fig.subplots()
    ax.set_title('Click on legend line to toggle line on/off')
    ax.plot(t, 2 * np.sin(2 * np.pi * t), lw=2, label='1 Hz')
    ax.plot(t, 4 * np.sin(2 * np.pi * 2 * t), lw=2, label='2 Hz')
    ax.legend(fancybox=True, shadow=True)


@tutorial("looking_glass")
def _(fig):
    x, y = _rows(rng(SEED), 2, 200)
    ax = fig.subplots()
    circ = Circle((0.5, 0.5), 0.25, alpha=0.8, fc='yellow')
    ax.add_patch(circ)
    ax.plot(x, y, alpha=0.2)
    ax.plot(x, y, alpha=1.0, clip_path=circ)
    ax.set_title("Left click and drag to move looking glass")


def _editor_path():
    """The path ``path_editor`` edits."""
    pathdata = [
        (Path.MOVETO, (1.58, -2.57)),
        (Path.CURVE4, (0.35, -1.1)),
        (Path.CURVE4, (-1.75, 2.0)),
        (Path.CURVE4, (0.375, 2.0)),
        (Path.LINETO, (0.85, 1.15)),
        (Path.CURVE4, (2.2, 3.2)),
        (Path.CURVE4, (3, 0.05)),
        (Path.CURVE4, (2.0, -0.5)),
        (Path.CLOSEPOLY, (1.58, -2.57)),
    ]
    codes, verts = zip(*pathdata)
    return Path(verts, codes)


@tutorial("path_editor")
def _(fig):
    ax = fig.subplots()
    patch = PathPatch(_editor_path(), facecolor='green', edgecolor='yellow', alpha=0.5)
    ax.add_patch(patch)
    x, y = zip(*patch.get_path().vertices)
    ax.plot(x, y, marker='o', markerfacecolor='r')
    ax.set_title('drag vertices to update path')
    ax.set_xlim(-3, 4)
    ax.set_ylim(-3, 4)


def _drag(fig, start, end):
    """Presses the left button at display point ``start``, moves the
    pointer to ``end`` and releases the button there."""
    for name, (x, y) in [("button_press_event", start), ("motion_notify_event", end),
                         ("button_release_event", end)]:
        MouseEvent(name, fig.canvas, x, y, button=MouseButton.LEFT)._process()


class _PathInteractor:
    """``path_editor``'s ``PathInteractor`` without its key binding: the
    patch and the vertex markers are animated, drawn by the draw callback
    over the background it saves, and dragging a vertex redraws them over
    that background and blits the axes."""

    epsilon = 5

    def __init__(self, pathpatch):
        self.ax = pathpatch.axes
        canvas = self.ax.figure.canvas
        self.pathpatch = pathpatch
        self.pathpatch.set_animated(True)
        x, y = zip(*self.pathpatch.get_path().vertices)
        self.line, = self.ax.plot(x, y, marker='o', markerfacecolor='r', animated=True)
        self._ind = None
        canvas.mpl_connect('draw_event', self.on_draw)
        canvas.mpl_connect('button_press_event', self.on_button_press)
        canvas.mpl_connect('button_release_event', self.on_button_release)
        canvas.mpl_connect('motion_notify_event', self.on_mouse_move)
        self.canvas = canvas

    def get_ind_under_point(self, event):
        xy = self.pathpatch.get_path().vertices
        xyt = self.pathpatch.get_transform().transform(xy)
        xt, yt = xyt[:, 0], xyt[:, 1]
        d = np.sqrt((xt - event.x)**2 + (yt - event.y)**2)
        ind = d.argmin()
        return ind if d[ind] < self.epsilon else None

    def on_draw(self, event):
        self.background = self.canvas.copy_from_bbox(self.ax.bbox)
        self.ax.draw_artist(self.pathpatch)
        self.ax.draw_artist(self.line)

    def on_button_press(self, event):
        if event.inaxes is None or event.button != MouseButton.LEFT:
            return
        self._ind = self.get_ind_under_point(event)

    def on_button_release(self, event):
        if event.button != MouseButton.LEFT:
            return
        self._ind = None

    def on_mouse_move(self, event):
        if self._ind is None or event.inaxes is None or event.button != MouseButton.LEFT:
            return
        vertices = self.pathpatch.get_path().vertices
        vertices[self._ind] = event.xdata, event.ydata
        self.line.set_data(zip(*vertices))
        self.canvas.restore_region(self.background)
        self.ax.draw_artist(self.pathpatch)
        self.ax.draw_artist(self.line)
        self.canvas.blit(self.ax.bbox)


@tutorial("path_editor_blitted", canvas=True)
def _(fig):
    """``path_editor`` as it runs, from its canvas: drawn, then its third
    vertex (-1.75, 2), at display point (168.6, 316.8), dragged to
    (250, 400)."""
    ax = fig.subplots()
    patch = PathPatch(_editor_path(), facecolor='green', edgecolor='yellow', alpha=0.5)
    ax.add_patch(patch)
    interactor = _PathInteractor(patch)
    ax.set_title('drag vertices to update path')
    ax.set_xlim(-3, 4)
    ax.set_ylim(-3, 4)
    fig.canvas.draw()
    _drag(fig, (169, 317), (250, 400))
    del interactor


@tutorial("pick_event_demo_1")
def _(fig):
    g = rng(SEED)
    ax1, ax2 = fig.subplots(2, 1)
    ax1.set_title('click on points, rectangles or text', picker=True)
    ax1.set_ylabel('ylabel', picker=True, bbox=dict(facecolor='red'))
    ax1.plot(g.uniform(100), 'o', picker=True, pickradius=5)
    ax2.bar(range(10), g.uniform(10), picker=True)


@tutorial("pick_event_demo_2")
def _(fig):
    g = rng(SEED)
    ax = fig.subplots()
    ax.set_title('custom picker for line data')
    ax.plot(g.uniform(100), g.uniform(100), 'o')


@tutorial("pick_event_demo_3")
def _(fig):
    x, y, c, s = _rows(rng(SEED), 4, 100)
    fig.subplots().scatter(x, y, 100*s, c, picker=True)


@tutorial("pick_event_demo_4")
def _(fig):
    g = rng(SEED)
    ax = fig.subplots()
    ax.imshow(_rows(g, 10, 5), extent=(1, 2, 1, 2), picker=True)
    ax.imshow(_rows(g, 5, 10), extent=(3, 4, 1, 2), picker=True)
    ax.imshow(_rows(g, 20, 25), extent=(1, 2, 3, 4), picker=True)
    ax.imshow(_rows(g, 30, 12), extent=(3, 4, 3, 4), picker=True)
    ax.set(xlim=(0, 5), ylim=(0, 5))


@tutorial("pick_event_demo2")
def _(fig):
    _means_and_deviations(fig.subplots(), 1000)


def _editor_circle():
    """The vertices of the polygon ``poly_editor`` edits."""
    theta = np.arange(0, 2*np.pi, 0.1)
    r = 1.5
    return np.column_stack([r * np.cos(theta), r * np.sin(theta)])


@tutorial("poly_editor")
def _(fig):
    poly = Polygon(_editor_circle())
    ax = fig.subplots()
    ax.add_patch(poly)
    x, y = zip(*poly.xy)
    ax.add_line(Line2D(x, y, marker='o', markerfacecolor='r'))
    ax.set_title('Click and drag a point to move it')
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)


class _PolygonInteractor:
    """``poly_editor``'s ``PolygonInteractor`` without its key bindings
    (and so without the property updates they cause): the markers are
    animated and, with the animated polygon, drawn by the draw callback
    over the background it saves; dragging a vertex redraws them over that
    background and blits the axes."""

    epsilon = 5

    def __init__(self, ax, poly):
        self.ax = ax
        canvas = poly.figure.canvas
        self.poly = poly
        x, y = zip(*self.poly.xy)
        self.line = Line2D(x, y, marker='o', markerfacecolor='r', animated=True)
        self.ax.add_line(self.line)
        self._ind = None
        canvas.mpl_connect('draw_event', self.on_draw)
        canvas.mpl_connect('button_press_event', self.on_button_press)
        canvas.mpl_connect('button_release_event', self.on_button_release)
        canvas.mpl_connect('motion_notify_event', self.on_mouse_move)
        self.canvas = canvas

    def on_draw(self, event):
        self.background = self.canvas.copy_from_bbox(self.ax.bbox)
        self.ax.draw_artist(self.poly)
        self.ax.draw_artist(self.line)

    def get_ind_under_point(self, event):
        xy = np.asarray(self.poly.xy)
        xyt = self.poly.get_transform().transform(xy)
        xt, yt = xyt[:, 0], xyt[:, 1]
        d = np.hypot(xt - event.x, yt - event.y)
        indseq, = np.nonzero(d == d.min())
        ind = indseq[0]
        if d[ind] >= self.epsilon:
            ind = None
        return ind

    def on_button_press(self, event):
        if event.inaxes is None or event.button != 1:
            return
        self._ind = self.get_ind_under_point(event)

    def on_button_release(self, event):
        if event.button != 1:
            return
        self._ind = None

    def on_mouse_move(self, event):
        if self._ind is None or event.inaxes is None or event.button != 1:
            return
        x, y = event.xdata, event.ydata
        self.poly.xy[self._ind] = x, y
        if self._ind == 0:
            self.poly.xy[-1] = x, y
        elif self._ind == len(self.poly.xy) - 1:
            self.poly.xy[0] = x, y
        self.line.set_data(zip(*self.poly.xy))
        self.canvas.restore_region(self.background)
        self.ax.draw_artist(self.poly)
        self.ax.draw_artist(self.line)
        self.canvas.blit(self.ax.bbox)


@tutorial("poly_editor_blitted", canvas=True)
def _(fig):
    """``poly_editor`` as it runs, from its canvas: drawn, then the
    vertex at angle 1, at display point (428.5, 354.2), dragged to
    (480, 420)."""
    poly = Polygon(_editor_circle(), animated=True)
    ax = fig.subplots()
    ax.add_patch(poly)
    interactor = _PolygonInteractor(ax, poly)
    ax.set_title('Click and drag a point to move it')
    ax.set_xlim((-2, 2))
    ax.set_ylim((-2, 2))
    fig.canvas.draw()
    _drag(fig, (428, 354), (480, 420))
    del interactor


@tutorial("resample")
def _(fig):
    xdata = np.linspace(16, 365, (365-16)*4)
    y1data = np.sin(2*np.pi*xdata/153) + np.cos(2*np.pi*xdata/127)
    # The whole range is visible: every 27th of the 1396 points is shown.
    x, y1, y2 = xdata[::27], y1data[::27], y1data[::27] + .2
    ax = fig.subplots()
    ax.plot(x, y1, 'o-')
    ax.fill_between(x, y1, y2, step="pre", color="r")
    ax.set_autoscale_on(False)
    ax.set_xlim(16, 365)


@tutorial("timers")
def _(fig):
    x = np.linspace(-3, 3)
    fig.subplots().plot(x, x ** 2)


@tutorial("trifinder_event_demo")
def _(fig):
    n_angles = 16
    n_radii = 5
    min_radius = 0.25
    radii = np.linspace(min_radius, 0.95, n_radii)
    angles = np.linspace(0, 2 * np.pi, n_angles, endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x = (radii*np.cos(angles)).flatten()
    y = (radii*np.sin(angles)).flatten()
    triang = Triangulation(x, y)
    triang.set_mask(np.hypot(x[triang.triangles].mean(axis=1),
                             y[triang.triangles].mean(axis=1))
                    < min_radius)
    ax = fig.subplots(subplot_kw={'aspect': 'equal'})
    ax.triplot(triang, 'bo-')
    ax.add_patch(Polygon(np.column_stack([x[[0, 0, 0]], y[[0, 0, 0]]]), facecolor='y'))


def _mandelbrot(xlim, ylim, width, height, niter=50, radius=2.):
    """For each point of a ``width`` x ``height`` grid over ``xlim`` x
    ``ylim``, the number of iterations it stays within ``radius``."""
    x = np.linspace(*xlim, width)
    y = np.linspace(*ylim, height).reshape(-1, 1)
    c = x + 1.0j * y
    threshold_time = np.zeros((height, width))
    z = np.zeros(threshold_time.shape, dtype=complex)
    mask = np.ones(threshold_time.shape, dtype=bool)
    for _ in range(niter):
        z[mask] = z[mask]**2 + c[mask]
        mask = (np.abs(z) < radius)
        threshold_time += mask
    return threshold_time


@tutorial("viewlims")
def _(fig):
    ax_full, ax_zoom = fig.subplots(1, 2)
    ax_zoom.imshow([[0]], origin="lower")
    ax_zoom.set_title("Zoom here")
    rect = Rectangle([0, 0], 0, 0, facecolor="none", edgecolor="black", linewidth=1.0)
    ax_full.add_patch(rect)
    # What the example's limit callbacks do when the zoom axes is set to
    # (-2, .5) x (-1.25, 1.25): track its bounds with the rectangle and
    # draw the set there, a pixel per point of the axes.
    ax_zoom.set_autoscale_on(False)
    ax_zoom.set(xlim=(-2, .5), ylim=(-1.25, 1.25))
    rect.set_bounds((-2, -1.25, 2.5, 2.5))
    width, height = ax_zoom.patch.get_window_extent().size.round().astype(int)
    im = ax_zoom.images[0]
    im.set(data=_mandelbrot((-2, .5), (-1.25, 1.25), width, height),
           extent=(-2, .5, -1.25, 1.25))
    im.set(clim=(im.get_array().min(), im.get_array().max()))
    ax_full.imshow(im.get_array(), extent=im.get_extent(), origin="lower")


def _zoom_window(fig, title, xlim, ylim):
    """``zoom_window``'s scatter, seen over ``xlim`` x ``ylim``."""
    x, y, s, c = _rows(rng(SEED), 4, 200)
    s *= 200
    ax = fig.subplots()
    ax.set(xlim=xlim, ylim=ylim, autoscale_on=False, title=title)
    ax.scatter(x, y, s, c)


@tutorial("zoom_window_1", figsize=(3.7, 3.7))
def _(fig):
    _zoom_window(fig, 'Click to zoom', (0, 1), (0, 1))


@tutorial("zoom_window_2", figsize=(3.7, 3.7))
def _(fig):
    _zoom_window(fig, 'Zoom window', (0.45, 0.55), (0.4, 0.6))
