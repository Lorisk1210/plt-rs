"""Blitting: figures drawn on their canvas, then changed by putting saved
regions back and drawing single artists over them (``FigureCanvasAgg``'s
``copy_from_bbox`` and ``restore_region``, ``draw_artist``,
``Axes.redraw_in_frame`` and ``FuncAnimation(blit=True)``).

Every case is a ``canvas`` case: its golden image is the canvas buffer after
the scripted draws, not the figure saved again, which would draw the
animated artists of every axes.

The figures of ``galleries/users_explain/animations/blitting.py`` (``minimal``
and ``blit_manager``) run as written after their 100 updates, with
``fig.canvas.draw()`` for ``plt.show(block=False)`` and ``plt.pause``; the
canvas's ``blit`` and ``flush_events`` do nothing on the Agg canvas. The
animations are stepped by firing their timer (``event_source._on_timer()``).
"""

import numpy as np
from matplotlib import animation
from matplotlib.patches import Circle
from matplotlib.transforms import Bbox

from spec_support import case

G = "blitting"
X = np.linspace(0, 2 * np.pi, 100)


def blitting(name, **options):
    """Registers a canvas case of this group."""
    return case(f"{G}/{name}", canvas=True, **options)


@blitting("minimal")
def _(fig):
    ax = fig.subplots()
    (ln,) = ax.plot(X, np.sin(X), animated=True)
    fig.canvas.draw()
    bg = fig.canvas.copy_from_bbox(fig.bbox)
    ax.draw_artist(ln)
    fig.canvas.blit(fig.bbox)
    for j in range(100):
        fig.canvas.restore_region(bg)
        ln.set_ydata(np.sin(X + (j / 100) * np.pi))
        ax.draw_artist(ln)
        fig.canvas.blit(fig.bbox)
        fig.canvas.flush_events()


class BlitManager:
    """The tutorial's ``BlitManager``: saves the background on every draw
    and draws its animated artists over it."""

    def __init__(self, canvas, animated_artists=()):
        self.canvas = canvas
        self._bg = None
        self._artists = []
        for a in animated_artists:
            self.add_artist(a)
        self.cid = canvas.mpl_connect("draw_event", self.on_draw)

    def on_draw(self, event):
        cv = self.canvas
        if event is not None:
            if event.canvas != cv:
                raise RuntimeError
        self._bg = cv.copy_from_bbox(cv.figure.bbox)
        self._draw_animated()

    def add_artist(self, art):
        if art.figure != self.canvas.figure:
            raise RuntimeError
        art.set_animated(True)
        self._artists.append(art)

    def _draw_animated(self):
        fig = self.canvas.figure
        for a in self._artists:
            fig.draw_artist(a)

    def update(self):
        cv = self.canvas
        fig = cv.figure
        if self._bg is None:
            self.on_draw(None)
        else:
            cv.restore_region(self._bg)
            self._draw_animated()
            cv.blit(fig.bbox)
        cv.flush_events()


@blitting("blit_manager")
def _(fig):
    ax = fig.subplots()
    (ln,) = ax.plot(X, np.sin(X), animated=True)
    fr_number = ax.annotate(
        "0",
        (0, 1),
        xycoords="axes fraction",
        xytext=(10, -10),
        textcoords="offset points",
        ha="left",
        va="top",
        animated=True,
    )
    bm = BlitManager(fig.canvas, [ln, fr_number])
    fig.canvas.draw()
    for j in range(100):
        ln.set_ydata(np.sin(X + (j / 100) * np.pi))
        fr_number.set_text(f"frame: {j}")
        bm.update()


@blitting("restore_region_bbox_xy")
def _(fig):
    """The axes scrolled 60 pixels to the left (``restore_region``'s
    docstring: ``bbox=(x1 + dx, y1, x2, y2), xy=(x1 - dx, y1)``, both in
    canvas pixels from the top), then the box around the legend copied to
    the top-left corner (``xy`` alone)."""
    ax = fig.subplots()
    ax.plot(X, np.sin(X), label="sin")
    ax.plot(X, np.cos(X), label="cos")
    ax.legend(loc="upper right")
    fig.canvas.draw()
    region = fig.canvas.copy_from_bbox(ax.bbox)
    x1, y1, x2, y2 = region.get_extents()
    dx = 60
    fig.canvas.restore_region(region, bbox=(x1 + dx, y1, x2, y2), xy=(x1 - dx, y1))
    corner = fig.canvas.copy_from_bbox(Bbox.from_bounds(480.5, 360.5, 100, 60))
    fig.canvas.restore_region(corner, xy=(10, 10))


def _overhanging(ax):
    """A line, a patch, a text and a collection reaching past the axes,
    animated."""
    (line,) = ax.plot([-0.3, 1.3], [0.1, 0.8], lw=4, animated=True)
    patch = ax.add_patch(Circle((0.9, 0.15), 0.3, fc="tab:orange", alpha=0.8, animated=True))
    text = ax.text(0.6, 0.55, "text past the edge", fontsize=14, clip_on=True, animated=True)
    dots = ax.scatter([0.05, 0.5, 0.97], [0.97, 1.0, 0.4], s=600,
                      c=["tab:green", "tab:red", "tab:purple"], animated=True)
    return [line, patch, text, dots]


def _overhanging_axes(fig):
    ax = fig.add_axes((0.2, 0.2, 0.6, 0.6))
    ax.set(xlim=(0, 1), ylim=(0, 1))
    return ax


@blitting("draw_artist_clipped", figsize=(4, 3))
def _(fig):
    ax = _overhanging_axes(fig)
    artists = _overhanging(ax)
    fig.canvas.draw()
    for artist in artists:
        ax.draw_artist(artist)


@blitting("draw_artist_unclipped", figsize=(4, 3))
def _(fig):
    """The artists of ``draw_artist_clipped`` with clipping off, and a text
    of the figure drawn by the figure."""
    ax = _overhanging_axes(fig)
    artists = _overhanging(ax)
    for artist in artists:
        artist.set_clip_on(False)
    note = fig.text(0.02, 0.03, "figure text", fontsize=12, animated=True)
    fig.canvas.draw()
    for artist in artists:
        ax.draw_artist(artist)
    fig.draw_artist(note)


@blitting("redraw_in_frame")
def _(fig):
    """After a new background colour, data and title, ``redraw_in_frame``
    draws the background, spines and line again; the title, ticks and
    labels keep their first drawing."""
    ax = fig.subplots()
    (line,) = ax.plot(X, np.sin(X))
    ax.set_title("before")
    ax.set_xlabel("x")
    fig.canvas.draw()
    ax.set_facecolor("lightyellow")
    line.set_ydata(np.cos(X))
    ax.set_title("after")
    ax.redraw_in_frame()


@blitting("func_animation")
def _(fig):
    """``simple_anim``, five frames in. The Agg canvas draws the figure as
    the animation is made, before the line becomes animated: the background
    saved at the first frame still shows the wave as it was."""
    ax = fig.subplots()
    (line,) = ax.plot(X, np.sin(X))

    def animate(i):
        line.set_ydata(np.sin(X + i / 50))
        return (line,)

    anim = animation.FuncAnimation(fig, animate, interval=20, blit=True, save_count=50)
    for _ in range(5):
        anim.event_source._on_timer()


@blitting("func_animation_init")
def _(fig):
    """A wave and a frame counter set up by ``init_func``, seven frames
    in."""
    ax = fig.subplots()
    ax.set(xlim=(0, 2 * np.pi), ylim=(-1.1, 1.1))
    (line,) = ax.plot([], [], lw=3)
    counter = ax.text(0.05, 0.9, "", transform=ax.transAxes)

    def init():
        line.set_data([], [])
        counter.set_text("")
        return line, counter

    def animate(i):
        line.set_data(X, np.sin(X + i / 10))
        counter.set_text(f"frame {i}")
        return line, counter

    anim = animation.FuncAnimation(fig, animate, init_func=init, frames=20, blit=True)
    for _ in range(7):
        anim.event_source._on_timer()
