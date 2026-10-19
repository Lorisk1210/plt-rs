"""matplotlib's own image tests from its remaining test modules:
``tests/test_multivariate_colormaps.py``, ``tests/test_sankey.py``,
``tests/test_png.py``, ``tests/test_ft2font.py``, ``tests/test_category.py``,
``tests/test_axis.py``, ``tests/test_animation.py``,
``tests/test_preprocess_data.py``, ``tests/test_backend_cairo.py`` (the
figure, rendered like every other case) and the ``fig_ref`` side of
``tests/test_pickle.py``: every ``@image_comparison`` test and the
``fig_test`` side of every ``@check_figures_equal`` test.

Each case is named after its test without the ``test_`` prefix; ``_ref``
cases contribute a test's ``fig_ref`` figure where it draws something the
``fig_test`` figure does not, and parametrized tests contribute one case per
parameter set. Tests without a style of their own draw under ``classic``
plus ``_classic_test_patch`` (matplotlib's test fixture applies it to
``check_figures_equal`` tests too) on classic's 8x6 in canvas saved at
100 dpi; ``mpl20`` and ``default`` need no rc. ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build. pyplot calls act on the
case's figure.

Deviations from the tests:

* ``test_pngsuite`` reads the fifteen basic images of PngSuite from
  matplotlib's baseline directory; the case reads fifteen PNG files of the
  same colour types and bit depths (grey of 1 to 16 bits, RGB, palettes of
  1 to 8 bits, grey + alpha and RGBA of 8 and 16 bits) whose 32x32 pixels
  are gradients given by formulas (:func:`_pngsuite`).
* ``test_animation_frame`` saves its animation through matplotlib's default
  movie writer to a temporary file; the case saves it with the Pillow
  writer, which needs no external program.

Not ported: ``test_data_kwarg`` (plt-rs has no ``data`` keyword; looking
the columns up draws the figures of ``test_dict_unpack``).
"""

import datetime
import io
import pathlib
import struct
import tempfile
import zlib

import matplotlib as mpl
import matplotlib.pyplot as plt
import matplotlib.style
import numpy as np
from matplotlib import animation
from matplotlib import collections as mcollections
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib.sankey import Sankey
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case

GROUP = "mpl_rest"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name, *, figsize=(8, 6)):
    """Register ``build`` as ``mpl_rest/<name>`` in the test suite's default
    style."""
    return case(f"{GROUP}/{name}", figsize=figsize, rc=CLASSIC)


def default(name, *, figsize=(6.4, 4.8)):
    """Register ``build`` as ``mpl_rest/<name>`` in matplotlib's default
    style (``mpl20``/``default``)."""
    return case(f"{GROUP}/{name}", figsize=figsize)


# --- test_multivariate_colormaps.py --------------------------------------


@classic("bivariate_cmap_shapes", figsize=(10, 2))
def _(fig):
    x_0 = np.repeat(np.linspace(-0.1, 1.1, 10, dtype="float32")[None, :], 10, axis=0)
    x_1 = x_0.T
    axes = fig.subplots(1, 4)
    shapes = [("BiPeak", "square"), ("BiCone", "circle"),
              ("BiPeak", "ignore"), ("BiCone", "circleignore")]
    for ax, (name, shape) in zip(axes, shapes):
        cmap = mpl.bivar_colormaps[name].with_extremes(shape=shape)
        ax.imshow(cmap((x_0, x_1)), interpolation="nearest")
    remove_ticks_and_titles(fig)


@classic("multivar_alpha_mixing")
def _(fig):
    rainbow = mpl.colormaps["rainbow"]
    alpha = np.zeros((256, 4))
    alpha[:, 3] = np.linspace(1, 0, 256)
    alpha_cmap = mpl.colors.LinearSegmentedColormap.from_list("from_list", alpha)
    cmap = mpl.colors.MultivarColormap((rainbow, alpha_cmap), "sRGB_add")
    y, x = np.mgrid[0:10, 0:10] / 9
    ax = fig.subplots()
    ax.imshow(cmap((y, x)), interpolation="nearest")
    remove_ticks_and_titles(fig)


# --- test_sankey.py ------------------------------------------------------


@classic("sankey3")
def _(fig):
    sankey = Sankey(ax=fig.gca(), flows=[0.25, -0.25, -0.25, 0.25, 0.5, -0.5],
                    orientations=[1, -1, 1, -1, 0, 0])
    sankey.finish()


# --- test_png.py ---------------------------------------------------------


def _chunk(tag, data):
    """One PNG chunk: length, type, data and CRC."""
    return (struct.pack(">I", len(data)) + tag + data
            + struct.pack(">I", zlib.crc32(tag + data)))


def _scanline(samples, bit_depth):
    """The bytes of one row of samples: packed most significant bits first
    below 8 bits, big-endian pairs at 16."""
    if bit_depth == 16:
        return b"".join(struct.pack(">H", s) for s in samples)
    per_byte = 8 // bit_depth
    packed = bytearray()
    for start in range(0, len(samples), per_byte):
        byte = 0
        for i, sample in enumerate(samples[start:start + per_byte]):
            byte |= sample << (8 - bit_depth * (i + 1))
        packed.append(byte)
    return bytes(packed)


def _png(color_type, bit_depth, pixel, palette=None):
    """A 32x32 PNG of colour type ``color_type`` whose pixel ``(x, y)`` has
    the samples ``pixel(x, y)``, unfiltered and not interlaced."""
    size = 32
    rows = (b"\0" + _scanline([s for x in range(size) for s in pixel(x, y)], bit_depth)
            for y in range(size))
    header = struct.pack(">IIBBBBB", size, size, bit_depth, color_type, 0, 0, 0)
    plte = _chunk(b"PLTE", bytes(c for rgb in palette for c in rgb)) if palette else b""
    return (b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", header) + plte
            + _chunk(b"IDAT", zlib.compress(b"".join(rows))) + _chunk(b"IEND", b""))


def _ramp(t, bit_depth):
    """``t`` in ``[0, 62]`` as a sample of ``bit_depth`` bits."""
    return t * (2**bit_depth - 1) // 62


def _pngsuite():
    """The stand-ins for PngSuite's ``basn*.png``, in file name order."""
    def grey(depth):
        return _png(0, depth, lambda x, y: [_ramp(x + y, depth)])

    def rgb(depth):
        return _png(2, depth, lambda x, y: [_ramp(2 * x, depth), _ramp(2 * y, depth),
                                            _ramp(62 - x - y, depth)])

    def palette(depth):
        n = 2**depth
        colors = [(k * 255 // (n - 1), 255 - k * 255 // (n - 1), k * 97 % 256)
                  for k in range(n)]
        return _png(3, depth, lambda x, y: [(x + y) * n // 64], palette=colors)

    def grey_alpha(depth):
        return _png(4, depth, lambda x, y: [_ramp(2 * x, depth), _ramp(2 * y, depth)])

    def rgba(depth):
        return _png(6, depth, lambda x, y: [_ramp(2 * x, depth), _ramp(2 * y, depth),
                                            _ramp(31, depth), _ramp(x + y, depth)])

    return ([grey(d) for d in (1, 2, 4, 8, 16)] + [rgb(d) for d in (8, 16)]
            + [palette(d) for d in (1, 2, 4, 8)]
            + [grey_alpha(d) for d in (8, 16)] + [rgba(d) for d in (8, 16)])


@default("pngsuite", figsize=(15, 2))
def _(fig):
    files = _pngsuite()
    ax = fig.gca()
    for i, png in enumerate(files):
        data = plt.imread(io.BytesIO(png))
        cmap = None  # use default colormap
        if data.ndim == 2:
            # keep grayscale images gray
            cmap = mpl.cm.gray
        ax.imshow(data, extent=(i, i + 1, 0, 1), cmap=cmap, interpolation="nearest")
    ax.patch.set_facecolor("#ddffff")
    ax.set_xlim(0, len(files))


# --- test_ft2font.py -----------------------------------------------------


@default("fallback_last_resort", figsize=(3, 0.5))
def _(fig):
    fig.text(.5, .5, "Hello 🙃 World!", size=24,
             horizontalalignment="center", verticalalignment="center")


# --- test_category.py ----------------------------------------------------


@default("overriding_units_in_plot")
def _(fig):
    t0 = datetime.datetime(2018, 3, 1)
    t1 = datetime.datetime(2018, 3, 2)
    t2 = datetime.datetime(2018, 3, 3)
    t3 = datetime.datetime(2018, 3, 4)
    ax = fig.subplots()
    ax.plot([t0, t1], ["V1", "V2"])
    ax.plot([t2, t3], ["V1", "V2"])


# --- test_axis.py --------------------------------------------------------


@classic("tick_not_in_layout")
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.add_subplot(xticks=[0, 1], xticklabels=["short", "very long"])
    ax.tick_params(labelcolor="w")
    fig.draw_without_rendering()  # Ensure ticks are correct.
    ax.xaxis.majorTicks[-1].label1.set_in_layout(False)


# --- test_animation.py ---------------------------------------------------


@classic("animation_frame")
def _(fig):
    ax = fig.add_subplot()
    ax.set_xlim(0, 2 * np.pi)
    ax.set_ylim(-1, 1)
    x = np.linspace(0, 2 * np.pi, 100)
    line, = ax.plot([], [])

    def init():
        line.set_data([], [])
        return line,

    def animate(i):
        line.set_data(x, np.sin(x + i / 100))
        return line,

    anim = animation.FuncAnimation(
        fig, animate, init_func=init, frames=5, blit=True, repeat=False)
    with tempfile.TemporaryDirectory() as tmp:
        anim.save(pathlib.Path(tmp) / "test.gif", writer="pillow")


# --- test_preprocess_data.py ---------------------------------------------


def _register_dict_unpack():
    """``TestPlotTypes.test_dict_unpack`` for each of ``Axes.scatter``,
    ``Axes.bar`` and ``Axes.plot``."""
    for plotter in ("scatter", "bar", "plot"):
        def dict_unpack(fig, plotter=plotter):
            ddict = dict(zip([1, 2, 3], [4, 5, 6]))
            getattr(fig.subplots(), plotter)(ddict.keys(), ddict.values())

        classic(f"dict_unpack_{plotter}")(dict_unpack)


_register_dict_unpack()


# --- test_backend_cairo.py -----------------------------------------------


def _cut_stars():
    """A unit circle with a six-pointed star cut out, and the same moved by
    ``(1, 1)``."""
    star = mpath.Path.unit_regular_star(6)
    circle = mpath.Path.unit_circle()
    verts = np.concatenate([circle.vertices, star.vertices[::-1]])
    codes = np.concatenate([circle.codes, star.codes])
    return mpath.Path(verts, codes), mpath.Path(verts + 1, codes)


#: The style both sides of ``test_patch_alpha_coloring`` give their shapes.
ALPHA_STYLE = {"linewidth": 5, "facecolor": (1, 0, 0, 0.5), "edgecolor": (0, 0, 1, 0.75)}


def _alpha_axes(fig):
    ax = fig.subplots()
    ax.set_xlim([-1, 2])
    ax.set_ylim([-1, 2])
    return ax


@classic("patch_alpha_coloring")
def _(fig):
    ax = _alpha_axes(fig)
    col = mcollections.PathCollection(list(_cut_stars()), linestyles="dashdot",
                                      **ALPHA_STYLE)
    ax.add_collection(col)


@classic("patch_alpha_coloring_ref")
def _(fig):
    ax = _alpha_axes(fig)
    for path in _cut_stars():
        ax.add_patch(mpatches.PathPatch(path, linestyle="dashdot", **ALPHA_STYLE))


# --- test_pickle.py ------------------------------------------------------


@default("complete_ref", figsize=(10, 6))
def _(fig):
    fig.suptitle("Can you fit any more in a figure?")

    # make some arbitrary data
    x, y = np.arange(8), np.arange(10)
    data = u = v = np.linspace(0, 10, 80).reshape(10, 8)
    v = np.sin(v * -0.6)

    ax = fig.add_subplot(3, 3, 1)
    ax.plot(list(range(10)))
    ax.set_ylabel("hello")

    ax = fig.add_subplot(3, 3, 2)
    contours = ax.contourf(data, hatches=["//", "ooo"])
    fig.colorbar(contours, ax=ax)

    ax = fig.add_subplot(3, 3, 3)
    ax.pcolormesh(data)

    ax = fig.add_subplot(3, 3, 4)
    ax.imshow(data)
    ax.set_ylabel("hello\nworld!")

    ax = fig.add_subplot(3, 3, 5)
    ax.pcolor(data)

    ax = fig.add_subplot(3, 3, 6)
    ax.set_xlim(0, 7)
    ax.set_ylim(0, 9)
    ax.streamplot(x, y, u, v)

    ax = fig.add_subplot(3, 3, 7)
    ax.set_xlim(0, 7)
    ax.set_ylim(0, 9)
    ax.quiver(x, y, u, v)

    ax = fig.add_subplot(3, 3, 8)
    ax.scatter(x, x ** 2, label="$x^2$")
    ax.legend(loc="upper left")

    ax = fig.add_subplot(3, 3, 9)
    ax.errorbar(x, x * -0.5, xerr=0.2, yerr=0.4, label="$-.5 x$")
    ax.legend(draggable=True)

    subfigs = fig.subfigures(2)
    subfigs[0].subplots(1, 2)
    subfigs[1].subplots(1, 2)

    fig.align_ylabels()
