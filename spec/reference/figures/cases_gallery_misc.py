"""The gallery's miscellaneous, specialty plot, showcase, spine and pyplot
sections (``galleries/examples/{misc,specialty_plots,showcase,spines,
pyplots}``), one case per figure (``<example>_<n>`` for examples drawing
several).

The examples run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng``, and the sample data files from synthetic
arrays of the same shape (``spec_support.gallery_data`` and the helpers
below).
pyplot calls act on the case's figure and its axes; ``plt.show()``,
``savefig`` and the SVG post-processing of the ``svg_filter`` examples are
dropped (the cases draw the figures those examples save). Wall-clock text
(``mandelbrot``'s year and version) is left out. ``custom_projection`` and
``skewt`` register their projections as the examples do; the axes class of
``radar_chart`` is passed as ``axes_class``, named like the polar
projection it extends, instead of being registered as a new projection.

Covered elsewhere: the first ``xkcd`` figure (``offsetbox/xkcd_bacon``),
``spines_dropped`` (``spines/dropped``), ``centered_spines_with_arrows``
(``spines/centered_with_arrows``, at 5 x 4 in), the second and third
``tickedstroke_demo`` figures (``gallery_lines/lines_with_ticks_demo`` and
``gallery_images/contours_in_optimization_demo``).

Not ported: ``font_indexing``, ``ftface_props`` and ``print_stdout`` (no
figure, or only bytes written to stdout), ``coords_report`` (its only
feature formats the interactive coordinate readout), ``multiprocess``
(interactive) and the second ``multipage_pdf`` page (usetex).
"""

import math
import re
from contextlib import ExitStack
from io import StringIO

import matplotlib as mpl
import matplotlib.axis as maxis
import matplotlib.pyplot as plt
import matplotlib.spines as mspines
import numpy as np
from matplotlib import cm
from matplotlib import colors as mcolors
from matplotlib import patheffects
from matplotlib import text as mtext
from matplotlib import transforms as mtransforms
from matplotlib.artist import Artist
from matplotlib.axes import Axes
from matplotlib.colors import LightSource, Normalize
from matplotlib.contour import ContourSet
from matplotlib.image import AxesImage
from matplotlib.lines import Line2D
from matplotlib.offsetbox import (AnchoredOffsetbox, AuxTransformBox, DrawingArea,
                                  TextArea, VPacker)
from matplotlib.patches import (Circle, Ellipse, PathPatch, Polygon, Rectangle,
                                RegularPolygon, Shadow, Wedge)
from matplotlib.path import Path
from matplotlib.projections import register_projection
from matplotlib.projections.polar import PolarAxes
from matplotlib.sankey import Sankey
from matplotlib.text import TextPath
from matplotlib.ticker import (AutoMinorLocator, FixedLocator, Formatter, MultipleLocator,
                               NullFormatter, NullLocator, ScalarFormatter)
from matplotlib.transforms import Affine2D, Bbox, BboxTransformTo, Transform, TransformedBbox

from spec_support import case, rng
from spec_support.gallery_data import contour_field, image_field, scan, terrain

G = "gallery_misc"
SEED = 19680801


# === misc =====================================================================


# --- anchored_artists ---------------------------------------------------------


@case(f"{G}/anchored_artists")
def _(fig):
    ax = fig.subplots()
    ax.set_aspect(1)
    box = AnchoredOffsetbox(child=TextArea("Figure 1a"), loc="upper left", frameon=True)
    box.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(box)
    area = DrawingArea(width=40, height=20)
    area.add_artist(Circle((10, 10), 10, fc="tab:blue"))
    area.add_artist(Circle((30, 10), 5, fc="tab:red"))
    ax.add_artist(AnchoredOffsetbox(child=area, loc="upper right", pad=0, frameon=False))
    aux_tr_box = AuxTransformBox(ax.transData)
    aux_tr_box.add_artist(Ellipse((0, 0), width=0.1, height=0.15))
    ax.add_artist(AnchoredOffsetbox(child=aux_tr_box, loc="lower left", frameon=True))
    sizebar = AuxTransformBox(ax.transData)
    sizebar.add_artist(Line2D([0, 0.1], [0, 0], color="black"))
    packer = VPacker(children=[sizebar, TextArea(r"1$^{\prime}$")], align="center", sep=5)
    ax.add_artist(AnchoredOffsetbox(child=packer, loc="lower center", frameon=False,
                                    pad=0.1, borderpad=0.5))


# --- bbox_intersect -----------------------------------------------------------


@case(f"{G}/bbox_intersect")
def _(fig):
    g = rng(SEED)
    left, bottom, width, height = (-1, -1, 2, 2)
    rect = plt.Rectangle((left, bottom), width, height, facecolor="black", alpha=0.1)
    ax = fig.subplots()
    ax.add_patch(rect)
    bbox = Bbox.from_bounds(left, bottom, width, height)
    for _ in range(12):
        vertices = (g.uniform(4).reshape(2, 2) - 0.5) * 6.0
        color = 'r' if Path(vertices).intersects_bbox(bbox) else 'b'
        ax.plot(vertices[:, 0], vertices[:, 1], color=color)


# --- contour_manual -----------------------------------------------------------


@case(f"{G}/contour_manual_1")
def _(fig):
    lines0 = [[[0, 0], [0, 4]]]
    lines1 = [[[2, 0], [1, 2], [1, 3]]]
    lines2 = [[[3, 0], [3, 2]], [[3, 3], [3, 4]]]
    filled01 = [[[0, 0], [0, 4], [1, 3], [1, 2], [2, 0]]]
    filled12 = [[[2, 0], [3, 0], [3, 2], [1, 3], [1, 2]], [[1, 4], [3, 4], [3, 3]]]
    ax = fig.subplots()
    cs = ContourSet(ax, [0, 1, 2], [filled01, filled12], filled=True, cmap="bone")
    cbar = fig.colorbar(cs)
    lines = ContourSet(ax, [0, 1, 2], [lines0, lines1, lines2], cmap="cool", linewidths=3)
    cbar.add_lines(lines)
    ax.set(xlim=(-0.5, 3.5), ylim=(-0.5, 4.5), title='User-specified contours')


@case(f"{G}/contour_manual_2")
def _(fig):
    ax = fig.subplots()
    filled01 = [[[0, 0], [3, 0], [3, 3], [0, 3], [1, 1], [1, 2], [2, 2], [2, 1]]]
    M, L = Path.MOVETO, Path.LINETO
    kinds01 = [[M, L, L, L, M, L, L, L]]
    cs = ContourSet(ax, [0, 1], [filled01], [kinds01], filled=True)
    fig.colorbar(cs)
    ax.set(xlim=(-0.5, 3.5), ylim=(-0.5, 3.5), title='User specified filled contours with holes')


# --- customize_rc -------------------------------------------------------------


@case(f"{G}/customize_rc")
def _(fig):
    fig.add_subplot(311).plot([1, 2, 3])
    custom = {
        "font.weight": "bold",
        "xtick.major.size": 5,
        "xtick.major.pad": 7,
        "xtick.labelsize": 15,
        "grid.color": "0.5",
        "grid.linestyle": "-",
        "grid.linewidth": 5,
        "lines.linewidth": 2,
        "lines.color": "g",
    }
    # ``plt.rcdefaults()`` returns to the reference configuration here.
    reference = {key: mpl.rcParams[key] for key in custom}
    mpl.rcParams.update(custom)
    ax = fig.add_subplot(312)
    ax.plot([1, 2, 3])
    ax.grid(True)
    mpl.rcParams.update(reference)
    ax = fig.add_subplot(313)
    ax.plot([1, 2, 3])
    ax.grid(True)


# --- custom_projection --------------------------------------------------------


class GeoAxes(Axes):
    """The example's abstract base class for geographic projections."""

    class ThetaFormatter(Formatter):
        def __init__(self, round_to=1.0):
            self._round_to = round_to

        def __call__(self, x, pos=None):
            degrees = round(np.rad2deg(x) / self._round_to) * self._round_to
            return f"{degrees:0.0f}\N{DEGREE SIGN}"

    RESOLUTION = 75

    def _init_axis(self):
        self.xaxis = maxis.XAxis(self)
        self.yaxis = maxis.YAxis(self)

    def clear(self):
        super().clear()
        self.set_longitude_grid(30)
        self.set_latitude_grid(15)
        self.set_longitude_grid_ends(75)
        self.xaxis.set_minor_locator(NullLocator())
        self.yaxis.set_minor_locator(NullLocator())
        self.xaxis.set_ticks_position('none')
        self.yaxis.set_ticks_position('none')
        self.yaxis.set_tick_params(label1On=True)
        self.grid(mpl.rcParams['axes.grid'])
        Axes.set_xlim(self, -np.pi, np.pi)
        Axes.set_ylim(self, -np.pi / 2.0, np.pi / 2.0)

    def _set_lim_and_transforms(self):
        self.transProjection = self._get_core_transform(self.RESOLUTION)
        self.transAffine = self._get_affine_transform()
        self.transAxes = BboxTransformTo(self.bbox)
        self.transData = self.transProjection + self.transAffine + self.transAxes
        self._xaxis_pretransform = (Affine2D().scale(1.0, self._longitude_cap * 2.0)
                                    .translate(0.0, -self._longitude_cap))
        self._xaxis_transform = self._xaxis_pretransform + self.transData
        self._xaxis_text1_transform = (Affine2D().scale(1.0, 0.0) + self.transData
                                       + Affine2D().translate(0.0, 4.0))
        self._xaxis_text2_transform = (Affine2D().scale(1.0, 0.0) + self.transData
                                       + Affine2D().translate(0.0, -4.0))
        yaxis_stretch = Affine2D().scale(np.pi*2, 1).translate(-np.pi, 0)
        yaxis_space = Affine2D().scale(1.0, 1.1)
        self._yaxis_transform = yaxis_stretch + self.transData
        yaxis_text_base = (yaxis_stretch + self.transProjection
                           + (yaxis_space + self.transAffine + self.transAxes))
        self._yaxis_text1_transform = yaxis_text_base + Affine2D().translate(-8.0, 0.0)
        self._yaxis_text2_transform = yaxis_text_base + Affine2D().translate(8.0, 0.0)

    def _get_affine_transform(self):
        transform = self._get_core_transform(1)
        xscale, _ = transform.transform((np.pi, 0))
        _, yscale = transform.transform((0, np.pi/2))
        return Affine2D().scale(0.5 / xscale, 0.5 / yscale).translate(0.5, 0.5)

    def get_xaxis_transform(self, which='grid'):
        if which not in ['tick1', 'tick2', 'grid']:
            raise ValueError("'which' must be one of 'tick1', 'tick2', or 'grid'")
        return self._xaxis_transform

    def get_xaxis_text1_transform(self, pad):
        return self._xaxis_text1_transform, 'bottom', 'center'

    def get_xaxis_text2_transform(self, pad):
        return self._xaxis_text2_transform, 'top', 'center'

    def get_yaxis_transform(self, which='grid'):
        if which not in ['tick1', 'tick2', 'grid']:
            raise ValueError("'which' must be one of 'tick1', 'tick2', or 'grid'")
        return self._yaxis_transform

    def get_yaxis_text1_transform(self, pad):
        return self._yaxis_text1_transform, 'center', 'right'

    def get_yaxis_text2_transform(self, pad):
        return self._yaxis_text2_transform, 'center', 'left'

    def _gen_axes_patch(self):
        return Circle((0.5, 0.5), 0.5)

    def _gen_axes_spines(self):
        return {'geo': mspines.Spine.circular_spine(self, (0.5, 0.5), 0.5)}

    def set_yscale(self, *args, **kwargs):
        if args[0] != 'linear':
            raise NotImplementedError

    set_xscale = set_yscale

    def set_xlim(self, *args, **kwargs):
        raise TypeError("Changing axes limits of a geographic projection is "
                        "not supported.  Please consider using Cartopy.")

    set_ylim = set_xlim

    def format_coord(self, lon, lat):
        lon, lat = np.rad2deg([lon, lat])
        ns = 'N' if lat >= 0.0 else 'S'
        ew = 'E' if lon >= 0.0 else 'W'
        return ('%f\N{DEGREE SIGN}%s, %f\N{DEGREE SIGN}%s'
                % (abs(lat), ns, abs(lon), ew))

    def set_longitude_grid(self, degrees):
        grid = np.arange(-180 + degrees, 180, degrees)
        self.xaxis.set_major_locator(FixedLocator(np.deg2rad(grid)))
        self.xaxis.set_major_formatter(self.ThetaFormatter(degrees))

    def set_latitude_grid(self, degrees):
        grid = np.arange(-90 + degrees, 90, degrees)
        self.yaxis.set_major_locator(FixedLocator(np.deg2rad(grid)))
        self.yaxis.set_major_formatter(self.ThetaFormatter(degrees))

    def set_longitude_grid_ends(self, degrees):
        self._longitude_cap = np.deg2rad(degrees)
        (self._xaxis_pretransform.clear().scale(1.0, self._longitude_cap * 2.0)
         .translate(0.0, -self._longitude_cap))

    def get_data_ratio(self):
        return 1.0

    def can_zoom(self):
        return False

    def can_pan(self):
        return False

    def start_pan(self, x, y, button):
        pass

    def end_pan(self):
        pass

    def drag_pan(self, button, key, x, y):
        pass


class HammerAxes(GeoAxes):
    """The example's Aitoff-Hammer projection, registered as
    ``custom_hammer``."""

    name = 'custom_hammer'

    class HammerTransform(Transform):
        input_dims = output_dims = 2

        def __init__(self, resolution):
            Transform.__init__(self)
            self._resolution = resolution

        def transform_non_affine(self, ll):
            longitude, latitude = ll.T
            half_long = longitude / 2
            cos_latitude = np.cos(latitude)
            sqrt2 = np.sqrt(2)
            alpha = np.sqrt(1 + cos_latitude * np.cos(half_long))
            x = (2 * sqrt2) * (cos_latitude * np.sin(half_long)) / alpha
            y = (sqrt2 * np.sin(latitude)) / alpha
            return np.column_stack([x, y])

        def transform_path_non_affine(self, path):
            ipath = path.interpolated(self._resolution)
            return Path(self.transform(ipath.vertices), ipath.codes)

        def inverted(self):
            return HammerAxes.InvertedHammerTransform(self._resolution)

    class InvertedHammerTransform(Transform):
        input_dims = output_dims = 2

        def __init__(self, resolution):
            Transform.__init__(self)
            self._resolution = resolution

        def transform_non_affine(self, xy):
            x, y = xy.T
            z = np.sqrt(1 - (x / 4) ** 2 - (y / 2) ** 2)
            longitude = 2 * np.arctan((z * x) / (2 * (2 * z ** 2 - 1)))
            latitude = np.arcsin(y*z)
            return np.column_stack([longitude, latitude])

        def inverted(self):
            return HammerAxes.HammerTransform(self._resolution)

    def __init__(self, *args, **kwargs):
        self._longitude_cap = np.pi / 2.0
        super().__init__(*args, **kwargs)
        self.set_aspect(0.5, adjustable='box', anchor='C')
        self.clear()

    def _get_core_transform(self, resolution):
        return self.HammerTransform(resolution)


register_projection(HammerAxes)


@case(f"{G}/custom_projection")
def _(fig):
    ax = fig.subplots(subplot_kw={'projection': 'custom_hammer'})
    ax.plot([-1, 1, 1], [-1, -1, 1], "o-")
    ax.grid()


# --- demo_agg_filter ----------------------------------------------------------


def _smooth1d(x, window_len):
    s = np.r_[2*x[0] - x[window_len:1:-1], x, 2*x[-1] - x[-1:-window_len:-1]]
    w = np.hanning(window_len)
    y = np.convolve(w/w.sum(), s, mode='same')
    return y[window_len-1:-window_len+1]


def _smooth2d(A, sigma=3):
    window_len = max(int(sigma), 3) * 2 + 1
    A = np.apply_along_axis(_smooth1d, 0, A, window_len)
    A = np.apply_along_axis(_smooth1d, 1, A, window_len)
    return A


class BaseFilter:
    def get_pad(self, dpi):
        return 0

    def process_image(self, padded_src, dpi):
        raise NotImplementedError("Should be overridden by subclasses")

    def __call__(self, im, dpi):
        pad = self.get_pad(dpi)
        padded_src = np.pad(im, [(pad, pad), (pad, pad), (0, 0)], "constant")
        tgt_image = self.process_image(padded_src, dpi)
        return tgt_image, -pad, -pad


class OffsetFilter(BaseFilter):
    def __init__(self, offsets=(0, 0)):
        self.offsets = offsets

    def get_pad(self, dpi):
        return int(max(self.offsets) / 72 * dpi)

    def process_image(self, padded_src, dpi):
        ox, oy = self.offsets
        a1 = np.roll(padded_src, int(ox / 72 * dpi), axis=1)
        a2 = np.roll(a1, -int(oy / 72 * dpi), axis=0)
        return a2


class GaussianFilter(BaseFilter):
    def __init__(self, sigma, alpha=0.5, color=(0, 0, 0)):
        self.sigma = sigma
        self.alpha = alpha
        self.color = color

    def get_pad(self, dpi):
        return int(self.sigma*3 / 72 * dpi)

    def process_image(self, padded_src, dpi):
        tgt_image = np.empty_like(padded_src)
        tgt_image[:, :, :3] = self.color
        tgt_image[:, :, 3] = _smooth2d(padded_src[:, :, 3] * self.alpha, self.sigma / 72 * dpi)
        return tgt_image


class DropShadowFilter(BaseFilter):
    def __init__(self, sigma, alpha=0.3, color=(0, 0, 0), offsets=(0, 0)):
        self.gauss_filter = GaussianFilter(sigma, alpha, color)
        self.offset_filter = OffsetFilter(offsets)

    def get_pad(self, dpi):
        return max(self.gauss_filter.get_pad(dpi), self.offset_filter.get_pad(dpi))

    def process_image(self, padded_src, dpi):
        t1 = self.gauss_filter.process_image(padded_src, dpi)
        return self.offset_filter.process_image(t1, dpi)


class LightFilter(BaseFilter):
    def __init__(self, sigma, fraction=1):
        self.gauss_filter = GaussianFilter(sigma, alpha=1)
        self.light_source = LightSource()
        self.fraction = fraction

    def get_pad(self, dpi):
        return self.gauss_filter.get_pad(dpi)

    def process_image(self, padded_src, dpi):
        t1 = self.gauss_filter.process_image(padded_src, dpi)
        elevation = t1[:, :, 3]
        rgb = padded_src[:, :, :3]
        alpha = padded_src[:, :, 3:]
        rgb2 = self.light_source.shade_rgb(rgb, elevation, fraction=self.fraction,
                                           blend_mode="overlay")
        return np.concatenate([rgb2, alpha], -1)


class GrowFilter(BaseFilter):
    def __init__(self, pixels, color=(1, 1, 1)):
        self.pixels = pixels
        self.color = color

    def __call__(self, im, dpi):
        alpha = np.pad(im[..., 3], self.pixels, "constant")
        alpha2 = np.clip(_smooth2d(alpha, self.pixels / 72 * dpi) * 5, 0, 1)
        new_im = np.empty((*alpha2.shape, 4))
        new_im[:, :, :3] = self.color
        new_im[:, :, 3] = alpha2
        return new_im, -self.pixels, -self.pixels


class FilteredArtistList(Artist):
    """A simple container to filter multiple artists at once."""

    def __init__(self, artist_list, filter):
        super().__init__()
        self._artist_list = artist_list
        self._filter = filter

    def draw(self, renderer):
        renderer.start_rasterizing()
        renderer.start_filter()
        for a in self._artist_list:
            a.draw(renderer)
        renderer.stop_filter(self._filter)
        renderer.stop_rasterizing()


def _filtered_text(ax):
    _, _, Z = contour_field()
    ax.imshow(Z, interpolation='bilinear', origin='lower', cmap="gray", extent=(-3, 3, -2, 2),
              aspect='auto')
    levels = np.arange(-1.2, 1.6, 0.2)
    CS = ax.contour(Z, levels, origin='lower', linewidths=2, extent=(-3, 3, -2, 2))
    cl = ax.clabel(CS, levels[1::2], fmt='%1.1f', fontsize=11)
    for t in cl:
        t.set_color("k")
        t.set_path_effects([patheffects.Normal()])
    white_glows = FilteredArtistList(cl, GrowFilter(3))
    ax.add_artist(white_glows)
    white_glows.set_zorder(cl[0].get_zorder() - 0.1)
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)


def _drop_shadow_line(ax):
    l1, = ax.plot([0.1, 0.5, 0.9], [0.1, 0.9, 0.5], "bo-")
    l2, = ax.plot([0.1, 0.5, 0.9], [0.5, 0.2, 0.7], "ro-")
    gauss = DropShadowFilter(4)
    for line in [l1, l2]:
        shadow, = ax.plot(line.get_xdata(), line.get_ydata())
        shadow.update_from(line)
        transform = mtransforms.offset_copy(line.get_transform(), ax.figure,
                                            x=4.0, y=-6.0, units='points')
        shadow.set_transform(transform)
        shadow.set_zorder(line.get_zorder() - 0.5)
        shadow.set_agg_filter(gauss)
        shadow.set_rasterized(True)
    ax.set_xlim(0., 1.)
    ax.set_ylim(0., 1.)
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)


def _drop_shadow_patches(ax):
    ind = np.arange(5)
    width = 0.35
    rects1 = ax.bar(ind, [20, 35, 30, 35, 27], width, color='r', ec="w", lw=2)
    rects2 = ax.bar(ind + width + 0.1, [25, 32, 34, 20, 25], width, color='y', ec="w", lw=2)
    shadow = FilteredArtistList(rects1 + rects2, DropShadowFilter(5, offsets=(1, 1)))
    ax.add_artist(shadow)
    shadow.set_zorder(rects1[0].get_zorder() - 0.1)
    ax.set_ylim(0, 40)
    ax.xaxis.set_visible(False)
    ax.yaxis.set_visible(False)


def _light_filter_pie(ax):
    pie = ax.pie([15, 30, 45, 10], explode=(0.1, 0.2, 0.1, 0.1))
    light_filter = LightFilter(9)
    for p in pie.wedges:
        p.set_agg_filter(light_filter)
        p.set_rasterized(True)
        p.set(ec="none", lw=2)
    shadow = FilteredArtistList(pie.wedges, DropShadowFilter(9, offsets=(3, -4), alpha=0.7))
    ax.add_artist(shadow)
    shadow.set_zorder(pie.wedges[0].get_zorder() - 0.1)


@case(f"{G}/demo_agg_filter")
def _(fig):
    axs = fig.subplots(2, 2)
    _filtered_text(axs[0, 0])
    _drop_shadow_line(axs[0, 1])
    _drop_shadow_patches(axs[1, 0])
    _light_filter_pie(axs[1, 1])
    axs[1, 1].set_frame_on(True)


# --- demo_ribbon_box ----------------------------------------------------------


def _present():
    """A 128 x 128 RGBA picture of a wrapped present (8 bit, divided by
    255) in place of ``Minduka_Present_Blue_Pack.png``: shaded like a
    cylinder, a pale ribbon down the middle, a tapering bow on top."""
    yy, xx = np.mgrid[0:128, 0:128]
    centre = np.abs(2 * xx - 127)
    blue = 255 - centre**2 * 160 // 16129
    red = np.where(centre < 24, blue, blue // 4)
    alpha = np.where((yy >= 24) | (centre < 3 * yy), 255, 0)
    return np.dstack([red, red, blue, alpha]) / 255


class RibbonBox:
    original_image = _present()
    cut_location = 70
    b_and_h = original_image[:, :, 2:3]
    color = original_image[:, :, 2:3] - original_image[:, :, 0:1]
    alpha = original_image[:, :, 3:4]
    nx = original_image.shape[1]

    def __init__(self, color):
        rgb = mcolors.to_rgb(color)
        self.im = np.dstack([self.b_and_h - self.color * (1 - np.array(rgb)), self.alpha])

    def get_stretched_image(self, stretch_factor):
        stretch_factor = max(stretch_factor, 1)
        ny, nx, nch = self.im.shape
        ny2 = int(ny*stretch_factor)
        return np.vstack([self.im[:self.cut_location],
                          np.broadcast_to(self.im[self.cut_location], (ny2 - ny, nx, nch)),
                          self.im[self.cut_location:]])


class RibbonBoxImage(AxesImage):
    zorder = 1

    def __init__(self, ax, bbox, color, *, extent=(0, 1, 0, 1), **kwargs):
        super().__init__(ax, extent=extent, **kwargs)
        self._bbox = bbox
        self._ribbonbox = RibbonBox(color)
        self.set_transform(BboxTransformTo(bbox))

    def draw(self, renderer):
        stretch_factor = self._bbox.height / self._bbox.width
        ny = int(stretch_factor*self._ribbonbox.nx)
        if self.get_array() is None or self.get_array().shape[0] != ny:
            self.set_array(self._ribbonbox.get_stretched_image(stretch_factor))
        super().draw(renderer)


@case(f"{G}/demo_ribbon_box")
def _(fig):
    ax = fig.subplots()
    years = np.arange(2004, 2009)
    heights = [7900, 8100, 7900, 6900, 2800]
    box_colors = [(0.8, 0.2, 0.2), (0.2, 0.8, 0.2), (0.2, 0.2, 0.8), (0.7, 0.5, 0.8),
                  (0.3, 0.8, 0.7)]
    for year, h, bc in zip(years, heights, box_colors):
        bbox0 = Bbox.from_extents(year - 0.4, 0., year + 0.4, h)
        bbox = TransformedBbox(bbox0, ax.transData)
        ax.add_artist(RibbonBoxImage(ax, bbox, bc, interpolation="bicubic"))
        ax.annotate(str(h), (year, h), va="bottom", ha="center")
    ax.set_xlim(years[0] - 0.5, years[-1] + 0.5)
    ax.set_ylim(0, 10000)
    background_gradient = np.zeros((2, 2, 4))
    background_gradient[:, :, :3] = [1, 1, 0]
    background_gradient[:, :, 3] = [[0.1, 0.3], [0.3, 0.5]]
    ax.imshow(background_gradient, interpolation="bicubic", zorder=0.1, extent=(0, 1, 0, 1),
              transform=ax.transAxes)


# --- fig_x, fill_spiral, findobj_demo -----------------------------------------


@case(f"{G}/fig_x")
def _(fig):
    fig.subplots(2, 2, gridspec_kw={'hspace': 0.4, 'wspace': 0.4})
    fig.add_artist(Line2D([0, 1], [0.47, 0.47], linewidth=3))
    fig.add_artist(Line2D([0.5, 0.5], [1, 0], linewidth=3))


@case(f"{G}/fill_spiral")
def _(fig):
    ax = fig.subplots()
    theta = np.arange(0, 8*np.pi, 0.1)
    a, b = 1, .2
    for dt in np.arange(0, 2*np.pi, np.pi/2.0):
        x = a*np.cos(theta + dt)*np.exp(b*theta)
        y = a*np.sin(theta + dt)*np.exp(b*theta)
        dt = dt + np.pi/4.0
        x2 = a*np.cos(theta + dt)*np.exp(b*theta)
        y2 = a*np.sin(theta + dt)*np.exp(b*theta)
        ax.fill(np.concatenate((x, x2[::-1])), np.concatenate((y, y2[::-1])))


@case(f"{G}/findobj_demo")
def _(fig):
    a = np.arange(0, 3, .02)
    c = np.exp(a)
    d = c[::-1]
    ax = fig.subplots()
    ax.plot(a, c, 'k--', a, d, 'k:', a, c + d, 'k')
    ax.legend(('Model length', 'Data length', 'Total message length'),
              loc='upper center', shadow=True)
    ax.set_ylim([-1, 20])
    ax.grid(False)
    ax.set_xlabel('Model complexity --->')
    ax.set_ylabel('Message length --->')
    ax.set_title('Minimum Message Length')
    for o in fig.findobj(lambda x: hasattr(x, 'set_color') and not hasattr(x, 'set_facecolor')):
        o.set_color('blue')
    for o in fig.findobj(mtext.Text):
        o.set_fontstyle('italic')


# --- histogram_path -----------------------------------------------------------


def _histogram_corners():
    data = rng(SEED).normal(1000)
    n, bins = np.histogram(data, 50)
    left = bins[:-1]
    right = bins[1:]
    bottom = np.zeros(len(left))
    top = bottom + n
    return left, right, bottom, top


def _add_histogram_patch(fig, barpath):
    patch = PathPatch(barpath)
    patch.sticky_edges.y[:] = [0]
    ax = fig.subplots()
    ax.add_patch(patch)
    ax.autoscale_view()


@case(f"{G}/histogram_path_1")
def _(fig):
    left, right, bottom, top = _histogram_corners()
    XY = np.array([[left, left, right, right], [bottom, top, top, bottom]]).T
    _add_histogram_patch(fig, Path.make_compound_path_from_polys(XY))


@case(f"{G}/histogram_path_2")
def _(fig):
    left, right, bottom, top = _histogram_corners()
    nverts = len(left) * 5
    verts = np.zeros((nverts, 2))
    codes = np.ones(nverts, int) * Path.LINETO
    codes[0::5] = Path.MOVETO
    codes[4::5] = Path.CLOSEPOLY
    verts[0::5, 0] = left
    verts[0::5, 1] = bottom
    verts[1::5, 0] = left
    verts[1::5, 1] = top
    verts[2::5, 0] = right
    verts[2::5, 1] = top
    verts[3::5, 0] = right
    verts[3::5, 1] = bottom
    _add_histogram_patch(fig, Path(verts, codes))


# --- hyperlinks ---------------------------------------------------------------


@case(f"{G}/hyperlinks_1")
def _(fig):
    s = fig.add_subplot().scatter([1, 2, 3], [4, 5, 6])
    s.set_urls(['https://www.bbc.com/news', 'https://www.google.com/', None])


@case(f"{G}/hyperlinks_2")
def _(fig):
    im = fig.add_subplot().imshow(image_field(), interpolation='bilinear', cmap="gray",
                                  origin='lower', extent=(-3, 3, -3, 3))
    im.set_url('https://www.google.com/')


# --- keyword_plotting ---------------------------------------------------------


@case(f"{G}/keyword_plotting")
def _(fig):
    g = rng(SEED)
    data = {'a': np.arange(50), 'c': g.integers(50, 0, 50), 'd': g.normal(50)}
    data['b'] = data['a'] + 10 * g.normal(50)
    data['d'] = np.abs(data['d']) * 100
    ax = fig.subplots()
    ax.scatter('a', 'b', c='c', s='d', data=data)
    ax.set(xlabel='entry a', ylabel='entry b')


# --- logos2 -------------------------------------------------------------------

MPL_BLUE = '#11557c'


def _icon_axes(fig, ax_position, lw_bars, lw_grid, lw_border, rgrid):
    with plt.rc_context({'axes.edgecolor': MPL_BLUE, 'axes.linewidth': lw_border}):
        ax = fig.add_axes(ax_position, projection='polar')
        ax.set_axisbelow(True)
        N = 7
        arc = 2. * np.pi
        theta = np.arange(0.0, arc, arc / N)
        radii = np.array([2, 6, 8, 7, 4, 5, 8])
        width = np.pi / 4 * np.array([0.4, 0.4, 0.6, 0.8, 0.2, 0.5, 0.3])
        bars = ax.bar(theta, radii, width=width, bottom=0.0, align='edge', edgecolor='0.3',
                      lw=lw_bars)
        for r, bar in zip(radii, bars):
            bar.set_facecolor((*cm.jet(r / 10.)[:3], 0.6))
        ax.tick_params(labelbottom=False, labeltop=False, labelleft=False, labelright=False)
        ax.grid(lw=lw_grid, color='0.9')
        ax.set_rmax(9)
        ax.set_yticks(rgrid)
        ax.add_patch(Rectangle((0, 0), arc, 9.58, facecolor='white', zorder=0, clip_on=False,
                               in_layout=False))


def _logo(fig, height_px, lw_bars, lw_grid, lw_border, rgrid, with_text=False):
    """``make_logo``. Neither Calibri nor Carlito is available, so the
    text is drawn in the default font (``get_font_properties()`` is
    ``None``)."""
    fig.patch.set_alpha(0)
    if with_text:
        ax = fig.add_axes((0, 0, 1, 1))
        ax.set_aspect("equal")
        ax.set_axis_off()
        path = TextPath((0, 0), "matplotlib", size=height_px * 0.8, prop=None)
        trans = mtransforms.Affine2D().skew_deg(4.25, 0)
        ax.add_patch(PathPatch(path, transform=trans + ax.transData, color=MPL_BLUE, lw=0))
        ax.autoscale()
    ax_pos = (0.535, 0.12, .17, 0.75) if with_text else (0.03, 0.03, .94, .94)
    _icon_axes(fig, ax_pos, lw_bars, lw_grid, lw_border, rgrid)


@case(f"{G}/logos2_1", figsize=(1.1, 1.1))
def _(fig):
    _logo(fig, height_px=110, lw_bars=0.7, lw_grid=0.5, lw_border=1, rgrid=[1, 3, 5, 7])


@case(f"{G}/logos2_2", figsize=(0.32, 0.32))
def _(fig):
    _logo(fig, height_px=32, lw_bars=0.3, lw_grid=0.3, lw_border=0.3, rgrid=[5])


@case(f"{G}/logos2_3", figsize=(5.5, 1.1))
def _(fig):
    _logo(fig, height_px=110, lw_bars=0.7, lw_grid=0.5, lw_border=1, rgrid=[1, 3, 5, 7],
          with_text=True)


# --- multipage_pdf ------------------------------------------------------------


@case(f"{G}/multipage_pdf_1", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(range(7), [3, 1, 4, 1, 5, 9, 2], 'r-o')
    ax.set_title('Page One')


@case(f"{G}/multipage_pdf_3", figsize=(4, 5))
def _(fig):
    x = np.arange(0, 5, 0.1)
    ax = fig.subplots()
    ax.plot(x, x ** 2, 'ko')
    ax.set_title('Page Three')


# --- packed_bubbles -----------------------------------------------------------


class BubbleChart:
    """The example's bubble packing, as written."""

    def __init__(self, area, bubble_spacing=0):
        area = np.asarray(area)
        r = np.sqrt(area / np.pi)
        self.bubble_spacing = bubble_spacing
        self.bubbles = np.ones((len(area), 4))
        self.bubbles[:, 2] = r
        self.bubbles[:, 3] = area
        self.maxstep = 2 * self.bubbles[:, 2].max() + self.bubble_spacing
        self.step_dist = self.maxstep / 2
        length = np.ceil(np.sqrt(len(self.bubbles)))
        grid = np.arange(length) * self.maxstep
        gx, gy = np.meshgrid(grid, grid)
        self.bubbles[:, 0] = gx.flatten()[:len(self.bubbles)]
        self.bubbles[:, 1] = gy.flatten()[:len(self.bubbles)]
        self.com = self.center_of_mass()

    def center_of_mass(self):
        return np.average(self.bubbles[:, :2], axis=0, weights=self.bubbles[:, 3])

    def center_distance(self, bubble, bubbles):
        return np.hypot(bubble[0] - bubbles[:, 0], bubble[1] - bubbles[:, 1])

    def outline_distance(self, bubble, bubbles):
        center_distance = self.center_distance(bubble, bubbles)
        return center_distance - bubble[2] - bubbles[:, 2] - self.bubble_spacing

    def check_collisions(self, bubble, bubbles):
        distance = self.outline_distance(bubble, bubbles)
        return len(distance[distance < 0])

    def collides_with(self, bubble, bubbles):
        distance = self.outline_distance(bubble, bubbles)
        return np.argmin(distance, keepdims=True)

    def collapse(self, n_iterations=50):
        for _i in range(n_iterations):
            moves = 0
            for i in range(len(self.bubbles)):
                rest_bub = np.delete(self.bubbles, i, 0)
                dir_vec = self.com - self.bubbles[i, :2]
                dir_vec = dir_vec / np.sqrt(dir_vec.dot(dir_vec))
                new_point = self.bubbles[i, :2] + dir_vec * self.step_dist
                new_bubble = np.append(new_point, self.bubbles[i, 2:4])
                if not self.check_collisions(new_bubble, rest_bub):
                    self.bubbles[i, :] = new_bubble
                    self.com = self.center_of_mass()
                    moves += 1
                else:
                    for colliding in self.collides_with(new_bubble, rest_bub):
                        dir_vec = rest_bub[colliding, :2] - self.bubbles[i, :2]
                        dir_vec = dir_vec / np.sqrt(dir_vec.dot(dir_vec))
                        orth = np.array([dir_vec[1], -dir_vec[0]])
                        new_point1 = self.bubbles[i, :2] + orth * self.step_dist
                        new_point2 = self.bubbles[i, :2] - orth * self.step_dist
                        dist1 = self.center_distance(self.com, np.array([new_point1]))
                        dist2 = self.center_distance(self.com, np.array([new_point2]))
                        new_point = new_point1 if dist1 < dist2 else new_point2
                        new_bubble = np.append(new_point, self.bubbles[i, 2:4])
                        if not self.check_collisions(new_bubble, rest_bub):
                            self.bubbles[i, :] = new_bubble
                            self.com = self.center_of_mass()
            if moves / len(self.bubbles) < 0.1:
                self.step_dist = self.step_dist / 2

    def plot(self, ax, labels, colors):
        for i in range(len(self.bubbles)):
            circ = plt.Circle(self.bubbles[i, :2], self.bubbles[i, 2], color=colors[i])
            ax.add_patch(circ)
            ax.text(*self.bubbles[i, :2], labels[i], horizontalalignment='center',
                    verticalalignment='center')


@case(f"{G}/packed_bubbles")
def _(fig):
    bubble_chart = BubbleChart(area=[8.61, 69.55, 8.36, 4.12, 2.76, 2.43], bubble_spacing=0.1)
    bubble_chart.collapse()
    ax = fig.subplots(subplot_kw=dict(aspect="equal"))
    bubble_chart.plot(ax, ['firefox', 'chrome', 'safari', 'edge', 'ie', 'opera'],
                      ['#5A69AF', '#579E65', '#F9C784', '#FC944A', '#F24C00', '#00B825'])
    ax.axis("off")
    ax.relim()
    ax.autoscale_view()
    ax.set_title('Browser market share')


# --- patheffect_demo ----------------------------------------------------------


@case(f"{G}/patheffect_demo", figsize=(8, 3))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.imshow([[1, 2], [2, 3]])
    txt = ax1.annotate("test", (1., 1.), (0., 0),
                       arrowprops=dict(arrowstyle="->", connectionstyle="angle3", lw=2),
                       size=20, ha="center",
                       path_effects=[patheffects.withStroke(linewidth=3, foreground="w")])
    txt.arrow_patch.set_path_effects([patheffects.Stroke(linewidth=5, foreground="w"),
                                      patheffects.Normal()])
    ax1.grid(True, linestyle="-",
             path_effects=[patheffects.withStroke(linewidth=3, foreground="w")])
    arr = np.arange(25).reshape((5, 5))
    ax2.imshow(arr)
    cntr = ax2.contour(arr, colors="k")
    cntr.set(path_effects=[patheffects.withStroke(linewidth=3, foreground="w")])
    clbls = ax2.clabel(cntr, fmt="%2.0f", use_clabeltext=True)
    plt.setp(clbls, path_effects=[patheffects.withStroke(linewidth=3, foreground="w")])
    p1, = ax3.plot([0, 1], [0, 1])
    leg = ax3.legend([p1], ["Line 1"], fancybox=True, loc='upper left')
    leg.legendPatch.set_path_effects([patheffects.withSimplePatchShadow()])


# --- rasterization_demo -------------------------------------------------------


@case(f"{G}/rasterization_demo")
def _(fig):
    d = np.arange(100).reshape(10, 10)
    x, y = np.meshgrid(np.arange(11), np.arange(11))
    theta = 0.25*np.pi
    xx = x*np.cos(theta) - y*np.sin(theta)
    yy = x*np.sin(theta) + y*np.cos(theta)
    fig.set_layout_engine("constrained")
    (ax1, ax2), (ax3, ax4) = fig.subplots(2, 2)
    ax1.set_aspect(1)
    ax1.pcolormesh(xx, yy, d)
    ax1.set_title("No Rasterization")
    ax2.set_aspect(1)
    ax2.set_title("Rasterization")
    ax2.pcolormesh(xx, yy, d, rasterized=True)
    ax3.set_aspect(1)
    ax3.pcolormesh(xx, yy, d)
    ax3.text(0.5, 0.5, "Text", alpha=0.2, va="center", ha="center", size=50,
             transform=ax3.transAxes)
    ax3.set_title("No Rasterization")
    ax4.set_aspect(1)
    ax4.pcolormesh(xx, yy, d, zorder=-10)
    ax4.text(0.5, 0.5, "Text", alpha=0.2, va="center", ha="center", size=50,
             transform=ax4.transAxes)
    ax4.set_rasterization_zorder(0)
    ax4.set_title("Rasterization z$<-10$")


# --- set_and_get --------------------------------------------------------------


@case(f"{G}/set_and_get")
def _(fig):
    x = np.arange(0, 1.0, 0.01)
    ax = fig.subplots()
    lines = ax.plot(x, np.sin(2*np.pi*x), x, np.sin(4*np.pi*x))
    l1, l2 = lines
    plt.setp(lines, linestyle='--')
    plt.setp(l1, linewidth=2, color='r')
    plt.setp(l2, linewidth=1, color='g')
    ax.set_title('Hi mom')


# --- svg_filter_line, svg_filter_pie ------------------------------------------


@case(f"{G}/svg_filter_line")
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    l1, = ax.plot([0.1, 0.5, 0.9], [0.1, 0.9, 0.5], "bo-", mec="b", lw=5, ms=10, label="Line 1")
    l2, = ax.plot([0.1, 0.5, 0.9], [0.5, 0.2, 0.7], "rs-", mec="r", lw=5, ms=10, label="Line 2")
    for line in [l1, l2]:
        shadow, = ax.plot(line.get_xdata(), line.get_ydata())
        shadow.update_from(line)
        shadow.set_color("0.2")
        shadow.set_zorder(line.get_zorder() - 0.5)
        shadow.set_transform(mtransforms.offset_copy(line.get_transform(), fig,
                                                     x=4.0, y=-6.0, units='points'))
        shadow.set_gid(line.get_label() + "_shadow")
    ax.set_xlim(0., 1.)
    ax.set_ylim(0., 1.)


@case(f"{G}/svg_filter_pie", figsize=(6, 6))
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    pie = ax.pie([15, 30, 45, 10], explode=(0, 0.05, 0, 0),
                 labels=('Frogs', 'Hogs', 'Dogs', 'Logs'), autopct='%1.1f%%')
    for w in pie.wedges:
        w.set_gid(w.get_label())
        w.set_edgecolor("none")
    for w in pie.wedges:
        s = Shadow(w, -0.01, -0.01)
        s.set_gid(w.get_gid() + "_shadow")
        s.set_zorder(w.get_zorder() - 0.1)
        ax.add_patch(s)


# --- table_demo ---------------------------------------------------------------


@case(f"{G}/table_demo")
def _(fig):
    data = [[66386, 174296, 75131, 577908, 32015],
            [58230, 381139, 78045, 99308, 160454],
            [89135, 80552, 152558, 497981, 603535],
            [78415, 81858, 150656, 193263, 69638],
            [139361, 331509, 343164, 781380, 52269]]
    columns = ('Freeze', 'Wind', 'Flood', 'Quake', 'Hail')
    rows = ['%d year' % x for x in (100, 50, 20, 10, 5)]
    values = np.arange(0, 2500, 500)
    value_increment = 1000
    colors = plt.colormaps["BuPu"](np.linspace(0, 0.5, len(rows)))
    index = np.arange(len(columns)) + 0.3
    bar_width = 0.4
    y_offset = np.zeros(len(columns))
    ax = fig.subplots()
    cell_text = []
    for row in range(len(data)):
        ax.bar(index, data[row], bar_width, bottom=y_offset, color=colors[row])
        y_offset = y_offset + data[row]
        cell_text.append(['%1.1f' % (x / 1000.0) for x in y_offset])
    colors = colors[::-1]
    cell_text.reverse()
    ax.table(cellText=cell_text, rowLabels=rows, rowColours=colors, colLabels=columns,
             loc='bottom')
    fig.subplots_adjust(left=0.2, bottom=0.2)
    ax.set_ylabel(f"Loss in ${value_increment}'s")
    ax.set_yticks(values * value_increment, ['%d' % val for val in values])
    ax.set_xticks([])
    ax.set_title('Loss by Disaster')


# --- tickedstroke_demo --------------------------------------------------------


@case(f"{G}/tickedstroke_demo_1", figsize=(6, 6))
def _(fig):
    ax = fig.subplots()
    patch = PathPatch(Path.unit_circle(), facecolor='none', lw=2, path_effects=[
        patheffects.withTickedStroke(angle=-90, spacing=10, length=1)])
    ax.add_patch(patch)
    ax.axis('equal')
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)


@case(f"{G}/tickedstroke_demo_4", figsize=(6, 6))
def _(fig):
    ax = fig.subplots()
    line_x = line_y = [0, 1]
    ax.plot(line_x, line_y, label="Line",
            path_effects=[patheffects.withTickedStroke(spacing=7, angle=135)])
    ax.plot(line_x, line_y, label="Opposite side",
            path_effects=[patheffects.withTickedStroke(spacing=7, angle=-135)])
    ax.legend()


# --- transoffset --------------------------------------------------------------


@case(f"{G}/transoffset", figsize=(5, 10))
def _(fig):
    xs = np.arange(7)
    ys = xs**2
    ax = fig.add_subplot(2, 1, 1)
    trans_offset = mtransforms.offset_copy(ax.transData, fig=fig, x=0.05, y=0.10,
                                           units='inches')
    for x, y in zip(xs, ys):
        ax.plot(x, y, 'ro')
        ax.text(x, y, '%d, %d' % (int(x), int(y)), transform=trans_offset)
    ax = fig.add_subplot(2, 1, 2, projection='polar')
    trans_offset = mtransforms.offset_copy(ax.transData, fig=fig, y=6, units='dots')
    for x, y in zip(xs, ys):
        ax.plot(x, y, 'ro')
        ax.text(x, y, '%d, %d' % (int(x), int(y)), transform=trans_offset,
                horizontalalignment='center', verticalalignment='bottom')


# --- zorder_demo --------------------------------------------------------------


@case(f"{G}/zorder_demo_1", figsize=(6, 3.2))
def _(fig):
    r = np.linspace(0.3, 1, 30)
    theta = np.linspace(0, 4*np.pi, 30)
    x = r * np.sin(theta)
    y = r * np.cos(theta)
    ax1, ax2 = fig.subplots(1, 2)
    ax1.plot(x, y, 'C3', lw=3)
    ax1.scatter(x, y, s=120)
    ax1.set_title('Lines on top of dots')
    ax2.plot(x, y, 'C3', lw=3)
    ax2.scatter(x, y, s=120, zorder=2.5)
    ax2.set_title('Dots on top of lines')
    fig.tight_layout()


@case(f"{G}/zorder_demo_2", rc={"lines.linewidth": 5})
def _(fig):
    x = np.linspace(0, 7.5, 100)
    ax = fig.subplots()
    ax.plot(x, np.sin(x), label='zorder=2', zorder=2)
    ax.plot(x, np.sin(x+0.5), label='zorder=3', zorder=3)
    ax.axhline(0, label='zorder=2.5', color='lightgrey', zorder=2.5)
    ax.set_title('Custom order of elements')
    legend = ax.legend(loc='upper right')
    legend.set_zorder(2.5)


# === specialty_plots ==========================================================


# --- advanced_hillshading -----------------------------------------------------


def _cos_rings():
    y, x = np.mgrid[-4:2:200j, -4:2:200j]
    return 10 * np.cos(x**2 + y**2)


@case(f"{G}/advanced_hillshading_1")
def _(fig):
    z = _cos_rings()
    cmap = plt.colormaps["copper"]
    rgb = LightSource(315, 45).shade(z, cmap)
    ax = fig.subplots()
    ax.imshow(rgb, interpolation='bilinear')
    im = ax.imshow(z, cmap=cmap)
    im.remove()
    fig.colorbar(im, ax=ax)
    ax.set_title('Using a colorbar with a shaded plot', size='x-large')


@case(f"{G}/advanced_hillshading_2", figsize=(8, 4.5))
def _(fig):
    z = _cos_rings()
    z[100, 105] = 2000
    z[120, 110] = -9000
    ls = LightSource(315, 45)
    ax1, ax2 = fig.subplots(ncols=2)
    ax1.imshow(ls.shade(z, plt.colormaps["copper"]), interpolation='bilinear')
    ax1.set_title('Full range of data')
    ax2.imshow(ls.shade(z, plt.colormaps["copper"], vmin=-10, vmax=10), interpolation='bilinear')
    ax2.set_title('Manually set range')
    fig.suptitle('Avoiding Outliers in Shaded Plots', size='x-large')


@case(f"{G}/advanced_hillshading_3")
def _(fig):
    y, x = np.mgrid[-4:2:200j, -4:2:200j]
    z1 = np.sin(x**2)
    z2 = np.cos(x**2 + y**2)
    norm = Normalize(z2.min(), z2.max())
    rgb = LightSource(315, 45).shade_rgb(plt.colormaps["RdBu"](norm(z2)), z1)
    ax = fig.subplots()
    ax.imshow(rgb, interpolation='bilinear')
    ax.set_title('Shade by one variable, color by another', size='x-large')


# --- anscombe -----------------------------------------------------------------


@case(f"{G}/anscombe", figsize=(6, 6))
def _(fig):
    x = [10, 8, 13, 9, 11, 14, 6, 4, 12, 7, 5]
    datasets = {
        'I': (x, [8.04, 6.95, 7.58, 8.81, 8.33, 9.96, 7.24, 4.26, 10.84, 4.82, 5.68]),
        'II': (x, [9.14, 8.14, 8.74, 8.77, 9.26, 8.10, 6.13, 3.10, 9.13, 7.26, 4.74]),
        'III': (x, [7.46, 6.77, 12.74, 7.11, 7.81, 8.84, 6.08, 5.39, 8.15, 6.42, 5.73]),
        'IV': ([8, 8, 8, 8, 8, 8, 8, 19, 8, 8, 8],
               [6.58, 5.76, 7.71, 8.84, 8.47, 7.04, 5.25, 12.50, 5.56, 7.91, 6.89]),
    }
    axs = fig.subplots(2, 2, sharex=True, sharey=True,
                       gridspec_kw={'wspace': 0.08, 'hspace': 0.08})
    axs[0, 0].set(xlim=(0, 20), ylim=(2, 14))
    axs[0, 0].set(xticks=(0, 10, 20), yticks=(4, 8, 12))
    for ax, (label, (x, y)) in zip(axs.flat, datasets.items()):
        ax.text(0.1, 0.9, label, fontsize=20, transform=ax.transAxes, va='top')
        ax.tick_params(direction='in', top=True, right=True)
        ax.plot(x, y, 'o')
        p1, p0 = np.polyfit(x, y, deg=1)
        ax.axline(xy1=(0, p0), slope=p1, color='r', lw=2)
        stats = (f'$\\mu$ = {np.mean(y):.2f}\n'
                 f'$\\sigma$ = {np.std(y):.2f}\n'
                 f'$r$ = {np.corrcoef(x, y)[0][1]:.2f}')
        bbox = dict(boxstyle='round', fc='blanchedalmond', ec='orange', alpha=0.5)
        ax.text(0.95, 0.07, stats, fontsize=9, bbox=bbox, transform=ax.transAxes,
                horizontalalignment='right')


# --- hinton_demo --------------------------------------------------------------


@case(f"{G}/hinton_demo")
def _(fig):
    matrix = rng(SEED).uniform(400).reshape(20, 20) - 0.5
    ax = fig.subplots()
    max_weight = 2 ** np.ceil(np.log2(np.abs(matrix).max()))
    ax.patch.set_facecolor('gray')
    ax.set_aspect('equal', 'box')
    ax.xaxis.set_major_locator(plt.NullLocator())
    ax.yaxis.set_major_locator(plt.NullLocator())
    for (x, y), w in np.ndenumerate(matrix):
        color = 'white' if w > 0 else 'black'
        size = np.sqrt(abs(w) / max_weight)
        ax.add_patch(plt.Rectangle([x - size / 2, y - size / 2], size, size,
                                   facecolor=color, edgecolor=color))
    ax.autoscale_view()
    ax.invert_yaxis()


# --- ishikawa_diagram ---------------------------------------------------------


def _ishikawa_problem(ax, data, problem_x, problem_y, angle_x, angle_y):
    ax.annotate(str.upper(data), xy=(problem_x, problem_y), xytext=(angle_x, angle_y),
                fontsize=10, color='white', weight='bold', xycoords='data',
                verticalalignment='center', horizontalalignment='center',
                textcoords='offset fontsize',
                arrowprops=dict(arrowstyle="->", facecolor='black'),
                bbox=dict(boxstyle='square', facecolor='tab:blue', pad=0.8))


def _ishikawa_causes(ax, data, cause_x, cause_y, cause_xytext=(-9, -0.3), top=True):
    coords = [[0.02, 0], [0.23, 0.5], [-0.46, -1], [0.69, 1.5], [-0.92, -2], [1.15, 2.5]]
    for index, cause in enumerate(data):
        cause_x -= coords[index][0]
        cause_y += coords[index][1] if top else -coords[index][1]
        ax.annotate(cause, xy=(cause_x, cause_y), horizontalalignment='center',
                    xytext=cause_xytext, fontsize=9, xycoords='data',
                    textcoords='offset fontsize',
                    arrowprops=dict(arrowstyle="->", facecolor='black'))


def _ishikawa_spine(ax, xmin, xmax):
    ax.plot([xmin - 0.1, xmax], [0, 0], color='tab:blue', linewidth=2)
    ax.text(xmax + 0.1, - 0.05, 'PROBLEM', fontsize=10, weight='bold', color='white')
    ax.add_patch(Wedge((xmax, 0), 1, 270, 90, fc='tab:blue'))
    tail_pos = [[xmin - 0.8, 0.8], [xmin - 0.8, -0.8], [xmin, -0.01]]
    ax.add_patch(Polygon(tail_pos, fc='tab:blue'))


@case(f"{G}/ishikawa_diagram", figsize=(10, 6))
def _(fig):
    fig.set_layout_engine("constrained")
    ax = fig.subplots()
    ax.set_xlim(-5, 5)
    ax.set_ylim(-5, 5)
    ax.axis('off')
    categories = {
        'Method': ['Time consumption', 'Cost', 'Procedures', 'Inefficient process', 'Sampling'],
        'Machine': ['Faulty equipment', 'Compatibility'],
        'Material': ['Poor-quality input', 'Raw materials', 'Supplier', 'Shortage'],
        'Measurement': ['Calibration', 'Performance', 'Wrong measurements'],
        'Environment': ['Bad conditions'],
        'People': ['Lack of training', 'Managers', 'Labor shortage', 'Procedures',
                   'Sales strategy'],
    }
    length = math.ceil(len(categories) / 2) - 1
    _ishikawa_spine(ax, -2 - length, 2 + length)
    offset = 0
    prob_section = [1.55, 0.8]
    for index, problem in enumerate(categories.values()):
        plot_above = index % 2 == 0
        cause_arrow_y = 1.7 if plot_above else -1.7
        y_prob_angle = 16 if plot_above else -16
        prob_arrow_x = prob_section[0] + length + offset
        cause_arrow_x = prob_section[1] + length + offset
        if not plot_above:
            offset -= 2.5
        _ishikawa_problem(ax, list(categories.keys())[index], prob_arrow_x, 0, -12,
                          y_prob_angle)
        _ishikawa_causes(ax, problem, cause_arrow_x, cause_arrow_y, top=plot_above)


# --- leftventricle_bullseye ---------------------------------------------------


def _bullseye_plot(ax, data, seg_bold=None, cmap="viridis", norm=None):
    data = np.ravel(data)
    if seg_bold is None:
        seg_bold = []
    if norm is None:
        norm = mpl.colors.Normalize(vmin=data.min(), vmax=data.max())
    r = np.linspace(0.2, 1, 4)
    ax.set(ylim=(0, 1), xticklabels=[], yticklabels=[])
    ax.grid(False)
    for start, stop, r_in, r_out in [(0, 6, r[2], r[3]), (6, 12, r[1], r[2]),
                                     (12, 16, r[0], r[1]), (16, 17, 0, r[0])]:
        n = stop - start
        dtheta = 2*np.pi / n
        ax.bar(np.arange(n) * dtheta + np.pi/2, r_out - r_in, dtheta, r_in,
               color=cmap(norm(data[start:stop])))
    for start, stop, r_in, r_out in [(0, 6, r[2], r[3]), (6, 12, r[1], r[2]),
                                     (12, 16, r[0], r[1])]:
        n = stop - start
        dtheta = 2*np.pi / n
        ax.bar(np.arange(n) * dtheta + np.pi/2, r_out - r_in, dtheta, r_in,
               clip_on=False, color="none", edgecolor="k",
               linewidth=[4 if i + 1 in seg_bold else 2 for i in range(start, stop)])
    ax.plot(np.linspace(0, 2*np.pi), np.linspace(r[0], r[0]), "k",
            linewidth=(4 if 17 in seg_bold else 2))


@case(f"{G}/leftventricle_bullseye", figsize=(10, 5))
def _(fig):
    data = np.arange(17) + 1
    fig.set_layout_engine("constrained")
    fig.get_layout_engine().set(wspace=.1, w_pad=.2)
    axs = fig.subplots(1, 3, subplot_kw=dict(projection='polar'))
    cmap = mpl.colormaps["viridis"]
    norm = mpl.colors.Normalize(vmin=1, vmax=17)
    fig.colorbar(mpl.cm.ScalarMappable(cmap=cmap, norm=norm),
                 cax=axs[0].inset_axes([0, -.15, 1, .1]),
                 orientation='horizontal', label='Some units')
    cmap2 = mpl.colormaps["cool"]
    norm2 = mpl.colors.Normalize(vmin=1, vmax=17)
    fig.colorbar(mpl.cm.ScalarMappable(cmap=cmap2, norm=norm2),
                 cax=axs[1].inset_axes([0, -.15, 1, .1]),
                 orientation='horizontal', label='Some other units')
    cmap3 = mpl.colors.ListedColormap(['r', 'g', 'b', 'c'], over='0.35', under='0.75')
    bounds = [2, 3, 7, 9, 15]
    norm3 = mpl.colors.BoundaryNorm(bounds, cmap3.N)
    fig.colorbar(mpl.cm.ScalarMappable(cmap=cmap3, norm=norm3),
                 cax=axs[2].inset_axes([0, -.15, 1, .1]),
                 extend='both', ticks=bounds, spacing='proportional',
                 orientation='horizontal', label='Discrete intervals, some other units')
    _bullseye_plot(axs[0], data, cmap=cmap, norm=norm)
    axs[0].set_title('Bulls Eye (AHA)')
    _bullseye_plot(axs[1], data, cmap=cmap2, norm=norm2)
    axs[1].set_title('Bulls Eye (AHA)')
    _bullseye_plot(axs[2], data, seg_bold=[3, 5, 6, 11, 12, 16], cmap=cmap3, norm=norm3)
    axs[2].set_title('Segments [3, 5, 6, 11, 12, 16] in bold')


# --- mri_with_eeg -------------------------------------------------------------


def _eeg():
    """800 samples of 4 EEG-like channels in place of ``eeg.dat``: unit
    noise on top of a slow rhythm per channel."""
    t = 10 * np.arange(800) / 800
    noise = rng(SEED).normal(3200, scale=0.8).reshape(800, 4)
    rhythm = np.column_stack([0.6 * np.sin(2 * np.pi * (1.5 + 0.5 * i) * t + i)
                              for i in range(4)])
    return noise + rhythm


@case(f"{G}/mri_with_eeg")
def _(fig):
    fig.set_layout_engine("constrained")
    axd = fig.subplot_mosaic([["image", "density"], ["EEG", "EEG"]], width_ratios=[1.05, 2])
    # A 16-bit slice like ``s1045.ima.gz``.
    im = (scan() * 55).astype(np.uint16)
    axd["image"].imshow(im, cmap="gray")
    axd["image"].axis('off')
    im = im[im.nonzero()]
    axd["density"].hist(im, bins=np.arange(0, 2**16+1, 512))
    axd["density"].set(xlabel='Intensity (a.u.)', xlim=(0, 2**16), ylabel='MRI density',
                       yticks=[])
    axd["density"].minorticks_on()
    n_samples, n_rows = 800, 4
    data = _eeg()
    t = 10 * np.arange(n_samples) / n_samples
    axd["EEG"].set_xlabel('Time (s)')
    axd["EEG"].set_xlim(0, 10)
    dy = (data.min() - data.max()) * 0.7
    axd["EEG"].set_ylim(-dy, n_rows * dy)
    axd["EEG"].set_yticks([0, dy, 2*dy, 3*dy], labels=['PG3', 'PG5', 'PG7', 'PG9'])
    for i, data_col in enumerate(data.T):
        axd["EEG"].plot(t, data_col + i*dy, color="C0")


# --- radar_chart --------------------------------------------------------------


def radar_factory(num_vars, frame='circle'):
    """The example's radar projection with ``num_vars`` axes, and its axes
    class. The example registers the class as ``radar``; the case passes
    the class itself instead, named like the polar axes it extends (which
    pltrs draws with a polygon frame), so that matplotlib's registry is
    left alone."""
    theta = np.linspace(0, 2*np.pi, num_vars, endpoint=False)

    class RadarTransform(PolarAxes.PolarTransform):
        def transform_path_non_affine(self, path):
            if path._interpolation_steps > 1:
                path = path.interpolated(num_vars)
            return Path(self.transform(path.vertices), path.codes)

    class RadarAxes(PolarAxes):
        name = 'polar'
        PolarTransform = RadarTransform

        def __init__(self, *args, **kwargs):
            super().__init__(*args, **kwargs)
            self.set_theta_zero_location('N')

        def fill(self, *args, closed=True, **kwargs):
            return super().fill(closed=closed, *args, **kwargs)

        def plot(self, *args, **kwargs):
            lines = super().plot(*args, **kwargs)
            for line in lines:
                self._close_line(line)

        def _close_line(self, line):
            x, y = line.get_data()
            if x[0] != x[-1]:
                x = np.append(x, x[0])
                y = np.append(y, y[0])
                line.set_data(x, y)

        def set_varlabels(self, labels):
            self.set_thetagrids(np.degrees(theta), labels)

        def _gen_axes_patch(self):
            if frame == 'circle':
                return Circle((0.5, 0.5), 0.5)
            return RegularPolygon((0.5, 0.5), num_vars, radius=.5, edgecolor="k")

        def _gen_axes_spines(self):
            if frame == 'circle':
                return super()._gen_axes_spines()
            spine = mspines.Spine(axes=self, spine_type='circle',
                                  path=Path.unit_regular_polygon(num_vars))
            spine.set_transform(Affine2D().scale(.5).translate(.5, .5) + self.transAxes)
            return {'polar': spine}

    return theta, RadarAxes


@case(f"{G}/radar_chart", figsize=(9, 9))
def _(fig):
    theta, RadarAxes = radar_factory(9, frame='polygon')
    spoke_labels = ['Sulfate', 'Nitrate', 'EC', 'OC1', 'OC2', 'OC3', 'OP', 'CO', 'O3']
    data = [
        ('Basecase', [
            [0.88, 0.01, 0.03, 0.03, 0.00, 0.06, 0.01, 0.00, 0.00],
            [0.07, 0.95, 0.04, 0.05, 0.00, 0.02, 0.01, 0.00, 0.00],
            [0.01, 0.02, 0.85, 0.19, 0.05, 0.10, 0.00, 0.00, 0.00],
            [0.02, 0.01, 0.07, 0.01, 0.21, 0.12, 0.98, 0.00, 0.00],
            [0.01, 0.01, 0.02, 0.71, 0.74, 0.70, 0.00, 0.00, 0.00]]),
        ('With CO', [
            [0.88, 0.02, 0.02, 0.02, 0.00, 0.05, 0.00, 0.05, 0.00],
            [0.08, 0.94, 0.04, 0.02, 0.00, 0.01, 0.12, 0.04, 0.00],
            [0.01, 0.01, 0.79, 0.10, 0.00, 0.05, 0.00, 0.31, 0.00],
            [0.00, 0.02, 0.03, 0.38, 0.31, 0.31, 0.00, 0.59, 0.00],
            [0.02, 0.02, 0.11, 0.47, 0.69, 0.58, 0.88, 0.00, 0.00]]),
        ('With O3', [
            [0.89, 0.01, 0.07, 0.00, 0.00, 0.05, 0.00, 0.00, 0.03],
            [0.07, 0.95, 0.05, 0.04, 0.00, 0.02, 0.12, 0.00, 0.00],
            [0.01, 0.02, 0.86, 0.27, 0.16, 0.19, 0.00, 0.00, 0.00],
            [0.01, 0.03, 0.00, 0.32, 0.29, 0.27, 0.00, 0.00, 0.95],
            [0.02, 0.00, 0.03, 0.37, 0.56, 0.47, 0.87, 0.00, 0.00]]),
        ('CO & O3', [
            [0.87, 0.01, 0.08, 0.00, 0.00, 0.04, 0.00, 0.00, 0.01],
            [0.09, 0.95, 0.02, 0.03, 0.00, 0.01, 0.13, 0.06, 0.00],
            [0.01, 0.02, 0.71, 0.24, 0.13, 0.16, 0.00, 0.50, 0.00],
            [0.01, 0.03, 0.00, 0.28, 0.24, 0.23, 0.00, 0.44, 0.88],
            [0.02, 0.00, 0.18, 0.45, 0.64, 0.55, 0.86, 0.00, 0.16]]),
    ]
    axs = fig.subplots(nrows=2, ncols=2, subplot_kw=dict(axes_class=RadarAxes))
    fig.subplots_adjust(wspace=0.25, hspace=0.20, top=0.85, bottom=0.05)
    colors = ['b', 'r', 'g', 'm', 'y']
    for ax, (title, case_data) in zip(axs.flat, data):
        ax.set_rgrids([0.2, 0.4, 0.6, 0.8])
        ax.set_title(title, weight='bold', size='medium', position=(0.5, 1.1),
                     horizontalalignment='center', verticalalignment='center')
        for d, color in zip(case_data, colors):
            ax.plot(theta, d, color=color)
            ax.fill(theta, d, facecolor=color, alpha=0.25, label='_nolegend_')
        ax.set_varlabels(spoke_labels)
    labels = ('Factor 1', 'Factor 2', 'Factor 3', 'Factor 4', 'Factor 5')
    axs[0, 0].legend(labels, loc=(0.9, .95), labelspacing=0.1, fontsize='small')
    fig.text(0.5, 0.965, '5-Factor Solution Profiles Across Four Scenarios',
             horizontalalignment='center', color='black', weight='bold', size='large')


# --- sankey_basics, sankey_links, sankey_rankine ------------------------------


def _sankey_axes(fig, title=None):
    """``fig.add_subplot(1, 1, 1, xticks=[], yticks=[], title=title)``, the
    axes ``Sankey`` makes when given none."""
    return fig.add_subplot(1, 1, 1, xticks=[], yticks=[], title=title)


@case(f"{G}/sankey_basics_1")
def _(fig):
    ax = _sankey_axes(fig)
    Sankey(ax=ax, flows=[0.25, 0.15, 0.60, -0.20, -0.15, -0.05, -0.50, -0.10],
           labels=['', '', '', 'First', 'Second', 'Third', 'Fourth', 'Fifth'],
           orientations=[-1, 1, 0, 1, 1, 1, 0, -1]).finish()
    ax.set_title("The default settings produce a diagram like this.")


@case(f"{G}/sankey_basics_2")
def _(fig):
    ax = _sankey_axes(fig, "Flow Diagram of a Widget")
    sankey = Sankey(ax=ax, scale=0.01, offset=0.2, head_angle=180, format='%.0f', unit='%')
    sankey.add(flows=[25, 0, 60, -10, -20, -5, -15, -10, -40],
               labels=['', '', '', 'First', 'Second', 'Third', 'Fourth', 'Fifth', 'Hurray!'],
               orientations=[-1, 1, 0, 1, 1, 1, -1, -1, 0],
               pathlengths=[0.25, 0.25, 0.25, 0.25, 0.25, 0.6, 0.25, 0.25, 0.25],
               patchlabel="Widget\nA")
    diagrams = sankey.finish()
    diagrams[0].texts[-1].set_color('r')
    diagrams[0].text.set_fontweight('bold')


@case(f"{G}/sankey_basics_3")
def _(fig):
    ax = _sankey_axes(fig, "Two Systems")
    flows = [0.25, 0.15, 0.60, -0.10, -0.05, -0.25, -0.15, -0.10, -0.35]
    sankey = Sankey(ax=ax, unit=None)
    sankey.add(flows=flows, label='one', orientations=[-1, 1, 0, 1, 1, 1, -1, -1, 0])
    sankey.add(flows=[-0.25, 0.15, 0.1], label='two', orientations=[-1, -1, -1], prior=0,
               connect=(0, 0))
    diagrams = sankey.finish()
    diagrams[-1].patch.set_hatch('/')
    ax.legend()


@case(f"{G}/sankey_links")
def _(fig):
    def side(sankey, n=1):
        prior = len(sankey.diagrams)
        for i in range(0, 2*n, 2):
            sankey.add(flows=[1, -1], orientations=[-1, -1], patchlabel=str(prior + i),
                       prior=prior + i - 1, connect=(1, 0), alpha=0.5)
            sankey.add(flows=[1, -1], orientations=[1, 1], patchlabel=str(prior + i + 1),
                       prior=prior + i, connect=(1, 0), alpha=0.5)

    def corner(sankey):
        prior = len(sankey.diagrams)
        sankey.add(flows=[1, -1], orientations=[0, 1], patchlabel=str(prior), facecolor='k',
                   prior=prior - 1, connect=(1, 0), alpha=0.5)

    ax = _sankey_axes(fig, "Why would you want to do this?\n(But you could.)")
    sankey = Sankey(ax=ax, unit=None)
    sankey.add(flows=[1, -1], orientations=[0, 1], patchlabel="0", facecolor='k', rotation=45)
    side(sankey, n=6)
    corner(sankey)
    side(sankey, n=6)
    corner(sankey)
    side(sankey, n=6)
    corner(sankey)
    side(sankey, n=6)
    sankey.finish()


@case(f"{G}/sankey_rankine", figsize=(8, 9))
def _(fig):
    ax = _sankey_axes(fig, "Rankine Power Cycle: Example 8.6 from Moran and "
                      "Shapiro\n\x22Fundamentals of Engineering Thermodynamics "
                      "\x22, 6th ed., 2008")
    Hdot = [260.431, 35.078, 180.794, 221.115, 22.700,
            142.361, 10.193, 10.210, 43.670, 44.312,
            68.631, 10.758, 10.758, 0.017, 0.642,
            232.121, 44.559, 100.613, 132.168]
    sankey = Sankey(ax=ax, format='%.3G', unit=' MW', gap=0.5, scale=1.0/Hdot[0])
    sankey.add(patchlabel='\n\nPump 1', rotation=90, facecolor='#37c959',
               flows=[Hdot[13], Hdot[6], -Hdot[7]], labels=['Shaft power', '', None],
               pathlengths=[0.4, 0.883, 0.25], orientations=[1, -1, 0])
    sankey.add(patchlabel='\n\nOpen\nheater', facecolor='#37c959',
               flows=[Hdot[11], Hdot[7], Hdot[4], -Hdot[8]], labels=[None, '', None, None],
               pathlengths=[0.25, 0.25, 1.93, 0.25], orientations=[1, 0, -1, 0], prior=0,
               connect=(2, 1))
    sankey.add(patchlabel='\n\nPump 2', facecolor='#37c959',
               flows=[Hdot[14], Hdot[8], -Hdot[9]], labels=['Shaft power', '', None],
               pathlengths=[0.4, 0.25, 0.25], orientations=[1, 0, 0], prior=1, connect=(3, 1))
    sankey.add(patchlabel='Closed\nheater', trunklength=2.914, fc='#37c959',
               flows=[Hdot[9], Hdot[1], -Hdot[11], -Hdot[10]],
               pathlengths=[0.25, 1.543, 0.25, 0.25], labels=['', '', None, None],
               orientations=[0, -1, 1, -1], prior=2, connect=(2, 0))
    sankey.add(patchlabel='Trap', facecolor='#37c959', trunklength=5.102,
               flows=[Hdot[11], -Hdot[12]], labels=['\n', None], pathlengths=[1.0, 1.01],
               orientations=[1, 1], prior=3, connect=(2, 0))
    sankey.add(patchlabel='Steam\ngenerator', facecolor='#ff5555',
               flows=[Hdot[15], Hdot[10], Hdot[2], -Hdot[3], -Hdot[0]],
               labels=['Heat rate', '', '', None, None], pathlengths=0.25,
               orientations=[1, 0, -1, -1, -1], prior=3, connect=(3, 1))
    sankey.add(patchlabel='\n\n\nTurbine 1', facecolor='#37c959',
               flows=[Hdot[0], -Hdot[16], -Hdot[1], -Hdot[2]],
               labels=['', None, None, None], pathlengths=[0.25, 0.153, 1.543, 0.25],
               orientations=[0, 1, -1, -1], prior=5, connect=(4, 0))
    sankey.add(patchlabel='\n\n\nReheat', facecolor='#37c959', flows=[Hdot[2], -Hdot[2]],
               labels=[None, None], pathlengths=[0.725, 0.25], orientations=[-1, 0], prior=6,
               connect=(3, 0))
    sankey.add(patchlabel='Turbine 2', trunklength=3.212, facecolor='#37c959',
               flows=[Hdot[3], Hdot[16], -Hdot[5], -Hdot[4], -Hdot[17]],
               labels=[None, 'Shaft power', None, '', 'Shaft power'],
               pathlengths=[0.751, 0.15, 0.25, 1.93, 0.25], orientations=[0, -1, 0, -1, 1],
               prior=6, connect=(1, 1))
    sankey.add(patchlabel='Condenser', facecolor='#58b1fa', trunklength=1.764,
               flows=[Hdot[5], -Hdot[18], -Hdot[6]], labels=['', 'Heat rate', None],
               pathlengths=[0.45, 0.25, 0.883], orientations=[-1, 1, 0], prior=8,
               connect=(2, 0))
    diagrams = sankey.finish()
    for diagram in diagrams:
        diagram.text.set_fontweight('bold')
        diagram.text.set_fontsize('10')
        for text in diagram.texts:
            text.set_fontsize('10')


# --- skewt --------------------------------------------------------------------


class SkewXTick(maxis.XTick):
    """The example's tick drawing its lower parts only within the lower x
    limits and its upper parts within the upper ones."""

    def draw(self, renderer):
        with ExitStack() as stack:
            for artist in [self.gridline, self.tick1line, self.tick2line,
                           self.label1, self.label2]:
                stack.callback(artist.set_visible, artist.get_visible())
            needs_lower = mtransforms._interval_contains(
                self.axes.lower_xlim, self.get_loc())
            needs_upper = mtransforms._interval_contains(
                self.axes.upper_xlim, self.get_loc())
            self.tick1line.set_visible(
                self.tick1line.get_visible() and needs_lower)
            self.label1.set_visible(
                self.label1.get_visible() and needs_lower)
            self.tick2line.set_visible(
                self.tick2line.get_visible() and needs_upper)
            self.label2.set_visible(
                self.label2.get_visible() and needs_upper)
            super().draw(renderer)

    def get_view_interval(self):
        return self.axes.xaxis.get_view_interval()


class SkewXAxis(maxis.XAxis):
    """The example's x axis ticking both the upper and the lower limits."""

    def _get_tick(self, major):
        return SkewXTick(self.axes, None, major=major)

    def get_view_interval(self):
        return self.axes.upper_xlim[0], self.axes.lower_xlim[1]


class SkewSpine(mspines.Spine):
    """The example's spine spanning the upper x limits at the top."""

    def _adjust_location(self):
        pts = self._path.vertices
        if self.spine_type == 'top':
            pts[:, 0] = self.axes.upper_xlim
        else:
            pts[:, 0] = self.axes.lower_xlim


class SkewXAxes(Axes):
    """The example's skewed x axes, registered as ``skewx``."""

    name = 'skewx'

    def _init_axis(self):
        self.xaxis = SkewXAxis(self)
        self.spines.top.register_axis(self.xaxis)
        self.spines.bottom.register_axis(self.xaxis)
        self.yaxis = maxis.YAxis(self)
        self.spines.left.register_axis(self.yaxis)
        self.spines.right.register_axis(self.yaxis)

    def _gen_axes_spines(self):
        spines = {'top': SkewSpine.linear_spine(self, 'top'),
                  'bottom': mspines.Spine.linear_spine(self, 'bottom'),
                  'left': mspines.Spine.linear_spine(self, 'left'),
                  'right': mspines.Spine.linear_spine(self, 'right')}
        return spines

    def _set_lim_and_transforms(self):
        rot = 30
        super()._set_lim_and_transforms()
        self.transDataToAxes = (
            self.transScale
            + self.transLimits
            + mtransforms.Affine2D().skew_deg(rot, 0)
        )
        self.transData = self.transDataToAxes + self.transAxes
        self._xaxis_transform = (
            mtransforms.blended_transform_factory(
                self.transScale + self.transLimits,
                mtransforms.IdentityTransform())
            + mtransforms.Affine2D().skew_deg(rot, 0)
            + self.transAxes
        )

    @property
    def lower_xlim(self):
        return self.axes.viewLim.intervalx

    @property
    def upper_xlim(self):
        pts = [[0., 1.], [1., 1.]]
        return self.transDataToAxes.inverted().transform(pts)[:, 0]


register_projection(SkewXAxes)

#: The example's sounding: pressure (hPa), height (m), temperature and dew
#: point (°C).
SKEWT_SOUNDING = '''
    978.0    345    7.8    0.8
    971.0    404    7.2    0.2
    946.7    610    5.2   -1.8
    944.0    634    5.0   -2.0
    925.0    798    3.4   -2.6
    911.8    914    2.4   -2.7
    906.0    966    2.0   -2.7
    877.9   1219    0.4   -3.2
    850.0   1478   -1.3   -3.7
    841.0   1563   -1.9   -3.8
    823.0   1736    1.4   -0.7
    813.6   1829    4.5    1.2
    809.0   1875    6.0    2.2
    798.0   1988    7.4   -0.6
    791.0   2061    7.6   -1.4
    783.9   2134    7.0   -1.7
    755.1   2438    4.8   -3.1
    727.3   2743    2.5   -4.4
    700.5   3048    0.2   -5.8
    700.0   3054    0.2   -5.8
    698.0   3077    0.0   -6.0
    687.0   3204   -0.1   -7.1
    648.9   3658   -3.2  -10.9
    631.0   3881   -4.7  -12.7
    600.7   4267   -6.4  -16.7
    592.0   4381   -6.9  -17.9
    577.6   4572   -8.1  -19.6
    555.3   4877  -10.0  -22.3
    536.0   5151  -11.7  -24.7
    533.8   5182  -11.9  -25.0
    500.0   5680  -15.9  -29.9
    472.3   6096  -19.7  -33.4
    453.0   6401  -22.4  -36.0
    400.0   7310  -30.7  -43.7
    399.7   7315  -30.8  -43.8
    387.0   7543  -33.1  -46.1
    382.7   7620  -33.8  -46.8
    342.0   8398  -40.5  -53.5
    320.4   8839  -43.7  -56.7
    318.0   8890  -44.1  -57.1
    310.0   9060  -44.7  -58.7
    306.1   9144  -43.9  -57.9
    305.0   9169  -43.7  -57.7
    300.0   9280  -43.5  -57.5
    292.0   9462  -43.7  -58.7
    276.0   9838  -47.1  -62.1
    264.0  10132  -47.5  -62.5
    251.0  10464  -49.7  -64.7
    250.0  10490  -49.7  -64.7
    247.0  10569  -48.7  -63.7
    244.0  10649  -48.9  -63.9
    243.3  10668  -48.9  -63.9
    220.0  11327  -50.3  -65.3
    212.0  11569  -50.5  -65.5
    210.0  11631  -49.7  -64.7
    200.0  11950  -49.9  -64.9
    194.0  12149  -49.9  -64.9
    183.0  12529  -51.3  -66.3
    164.0  13233  -55.3  -68.3
    152.0  13716  -56.5  -69.5
    150.0  13800  -57.1  -70.1
    136.0  14414  -60.5  -72.5
    132.0  14600  -60.1  -72.1
    131.4  14630  -60.2  -72.2
    128.0  14792  -60.9  -72.9
    125.0  14939  -60.1  -72.1
    119.0  15240  -62.2  -73.8
    112.0  15616  -64.9  -75.9
    108.0  15838  -64.1  -75.1
    107.8  15850  -64.1  -75.1
    105.0  16010  -64.7  -75.7
    103.0  16128  -62.9  -73.9
    100.0  16310  -62.5  -73.5
'''


@case(f"{G}/skewt", figsize=(6.5875, 6.2125))
def _(fig):
    p, h, T, Td = np.loadtxt(StringIO(SKEWT_SOUNDING), unpack=True)
    ax = fig.add_subplot(projection='skewx')
    plt.grid(True)
    ax.semilogy(T, p, color='C3')
    ax.semilogy(Td, p, color='C2')
    ax.axvline(0, color='C0')
    ax.yaxis.set_major_formatter(ScalarFormatter())
    ax.yaxis.set_minor_formatter(NullFormatter())
    ax.set_yticks(np.linspace(100, 1000, 10))
    ax.set_ylim(1050, 100)
    ax.xaxis.set_major_locator(MultipleLocator(10))
    ax.set_xlim(-50, 50)


# --- topographic_hillshading --------------------------------------------------


@case(f"{G}/topographic_hillshading", figsize=(8, 9))
def _(fig):
    # The cell size and latitude of ``jacksboro_fault_dem.npz``.
    z = terrain()
    dx = dy = 0.000833333333333
    dy = 111200 * dy
    dx = 111200 * dx * np.cos(np.radians(36.7329166666667))
    ls = LightSource(azdeg=315, altdeg=45)
    cmap = plt.colormaps["gist_earth"]
    axs = fig.subplots(nrows=4, ncols=3)
    plt.setp(axs.flat, xticks=[], yticks=[])
    for col, ve in zip(axs.T, [0.1, 1, 10]):
        col[0].imshow(ls.hillshade(z, vert_exag=ve, dx=dx, dy=dy), cmap='gray')
        for ax, mode in zip(col[1:], ['hsv', 'overlay', 'soft']):
            rgb = ls.shade(z, cmap=cmap, blend_mode=mode, vert_exag=ve, dx=dx, dy=dy)
            ax.imshow(rgb)
    for ax, ve in zip(axs[0], [0.1, 1, 10]):
        ax.set_title(f'{ve}', size=18)
    for ax, mode in zip(axs[:, 0], ['Hillshade', 'hsv', 'overlay', 'soft']):
        ax.set_ylabel(mode, size=18)
    axs[0, 1].annotate('Vertical Exaggeration', (0.5, 1), xytext=(0, 30),
                       textcoords='offset points', xycoords='axes fraction',
                       ha='center', va='bottom', size=20)
    axs[2, 0].annotate('Blend Mode', (0, 0.5), xytext=(-30, 0),
                       textcoords='offset points', xycoords='axes fraction',
                       ha='right', va='center', size=20, rotation=90)
    fig.subplots_adjust(bottom=0.05, right=0.95)


# === showcase =================================================================


# --- anatomy ------------------------------------------------------------------


@case(f"{G}/anatomy", figsize=(7.5, 7.5))
def _(fig):
    royal_blue = [0, 20/256, 82/256]
    X = np.linspace(0.5, 3.5, 100)
    Y1 = 3+np.cos(X)
    Y2 = 1+np.cos(1+X/0.75)/2
    Y3 = Y1 + (Y2 - Y1) * rng(SEED).uniform(len(X))
    ax = fig.add_axes((0.2, 0.17, 0.68, 0.7), aspect=1)
    ax.xaxis.set_major_locator(MultipleLocator(1.000))
    ax.xaxis.set_minor_locator(AutoMinorLocator(4))
    ax.yaxis.set_major_locator(MultipleLocator(1.000))
    ax.yaxis.set_minor_locator(AutoMinorLocator(4))
    ax.xaxis.set_minor_formatter("{x:.2f}")
    ax.set_xlim(0, 4)
    ax.set_ylim(0, 4)
    ax.tick_params(which='major', width=1.0, length=10, labelsize=14)
    ax.tick_params(which='minor', width=1.0, length=5, labelsize=10, labelcolor='0.25')
    ax.grid(linestyle="--", linewidth=0.5, color='.25', zorder=-10)
    ax.plot(X, Y1, c='C0', lw=2.5, label="Blue signal", zorder=10)
    ax.plot(X, Y2, c='C1', lw=2.5, label="Orange signal")
    ax.plot(X[::3], Y3[::3], linewidth=0, markersize=9, marker='s', markerfacecolor='none',
            markeredgecolor='C4', markeredgewidth=2.5)
    ax.set_title("Anatomy of a figure", fontsize=20, verticalalignment='bottom')
    ax.set_xlabel("x Axis label", fontsize=14)
    ax.set_ylabel("y Axis label", fontsize=14)
    ax.legend(loc="upper right", fontsize=14)

    def annotate(x, y, text, code):
        c = Circle((x, y), radius=0.15, clip_on=False, zorder=10, linewidth=2.5,
                   edgecolor=royal_blue + [0.6], facecolor='none',
                   path_effects=[patheffects.withStroke(linewidth=7, foreground='white')])
        ax.add_artist(c)
        for path_effects in [[patheffects.withStroke(linewidth=7, foreground='white')], []]:
            color = 'white' if path_effects else royal_blue
            ax.text(x, y-0.2, text, zorder=100, ha='center', va='top', weight='bold',
                    color=color, style='italic', fontfamily='monospace',
                    path_effects=path_effects)
            color = 'white' if path_effects else 'black'
            ax.text(x, y-0.33, code, zorder=100, ha='center', va='top', weight='normal',
                    color=color, fontfamily='monospace', fontsize='medium',
                    path_effects=path_effects)

    annotate(3.5, -0.13, "Minor tick label", "ax.xaxis.set_minor_formatter")
    annotate(-0.03, 1.0, "Major tick", "ax.yaxis.set_major_locator")
    annotate(0.00, 3.75, "Minor tick", "ax.yaxis.set_minor_locator")
    annotate(-0.15, 3.00, "Major tick label", "ax.yaxis.set_major_formatter")
    annotate(1.68, -0.39, "xlabel", "ax.set_xlabel")
    annotate(-0.38, 1.67, "ylabel", "ax.set_ylabel")
    annotate(1.52, 4.15, "Title", "ax.set_title")
    annotate(1.75, 2.80, "Line", "ax.plot")
    annotate(2.25, 1.54, "Markers", "ax.scatter")
    annotate(3.00, 3.00, "Grid", "ax.grid")
    annotate(3.60, 3.58, "Legend", "ax.legend")
    annotate(2.5, 0.55, "Axes", "fig.subplots")
    annotate(4, 4.5, "Figure", "plt.figure")
    annotate(0.65, 0.01, "x Axis", "ax.xaxis")
    annotate(0, 0.36, "y Axis", "ax.yaxis")
    annotate(4.0, 0.7, "Spine", "ax.spines")
    fig.patch.set(linewidth=4, edgecolor='0.5')


# --- firefox ------------------------------------------------------------------

FIREFOX = "M28.4,22.469c0.479-0.964,0.851-1.991,1.095-3.066c0.953-3.661,0.666-6.854,0.666-6.854l-0.327,2.104c0,0-0.469-3.896-1.044-5.353c-0.881-2.231-1.273-2.214-1.274-2.21c0.542,1.379,0.494,2.169,0.483,2.288c-0.01-0.016-0.019-0.032-0.027-0.047c-0.131-0.324-0.797-1.819-2.225-2.878c-2.502-2.481-5.943-4.014-9.745-4.015c-4.056,0-7.705,1.745-10.238,4.525C5.444,6.5,5.183,5.938,5.159,5.317c0,0-0.002,0.002-0.006,0.005c0-0.011-0.003-0.021-0.003-0.031c0,0-1.61,1.247-1.436,4.612c-0.299,0.574-0.56,1.172-0.777,1.791c-0.375,0.817-0.75,2.004-1.059,3.746c0,0,0.133-0.422,0.399-0.988c-0.064,0.482-0.103,0.971-0.116,1.467c-0.09,0.845-0.118,1.865-0.039,3.088c0,0,0.032-0.406,0.136-1.021c0.834,6.854,6.667,12.165,13.743,12.165l0,0c1.86,0,3.636-0.37,5.256-1.036C24.938,27.771,27.116,25.196,28.4,22.469zM16.002,3.356c2.446,0,4.73,0.68,6.68,1.86c-2.274-0.528-3.433-0.261-3.423-0.248c0.013,0.015,3.384,0.589,3.981,1.411c0,0-1.431,0-2.856,0.41c-0.065,0.019,5.242,0.663,6.327,5.966c0,0-0.582-1.213-1.301-1.42c0.473,1.439,0.351,4.17-0.1,5.528c-0.058,0.174-0.118-0.755-1.004-1.155c0.284,2.037-0.018,5.268-1.432,6.158c-0.109,0.07,0.887-3.189,0.201-1.93c-4.093,6.276-8.959,2.539-10.934,1.208c1.585,0.388,3.267,0.108,4.242-0.559c0.982-0.672,1.564-1.162,2.087-1.047c0.522,0.117,0.87-0.407,0.464-0.872c-0.405-0.466-1.392-1.105-2.725-0.757c-0.94,0.247-2.107,1.287-3.886,0.233c-1.518-0.899-1.507-1.63-1.507-2.095c0-0.366,0.257-0.88,0.734-1.028c0.58,0.062,1.044,0.214,1.537,0.466c0.005-0.135,0.006-0.315-0.001-0.519c0.039-0.077,0.015-0.311-0.047-0.596c-0.036-0.287-0.097-0.582-0.19-0.851c0.01-0.002,0.017-0.007,0.021-0.021c0.076-0.344,2.147-1.544,2.299-1.659c0.153-0.114,0.55-0.378,0.506-1.183c-0.015-0.265-0.058-0.294-2.232-0.286c-0.917,0.003-1.425-0.894-1.589-1.245c0.222-1.231,0.863-2.11,1.919-2.704c0.02-0.011,0.015-0.021-0.008-0.027c0.219-0.127-2.524-0.006-3.76,1.604C9.674,8.045,9.219,7.95,8.71,7.95c-0.638,0-1.139,0.07-1.603,0.187c-0.05,0.013-0.122,0.011-0.208-0.001C6.769,8.04,6.575,7.88,6.365,7.672c0.161-0.18,0.324-0.356,0.495-0.526C9.201,4.804,12.43,3.357,16.002,3.356z"  # noqa: E501


def _svg_parse(path):
    commands = {'M': (Path.MOVETO,),
                'L': (Path.LINETO,),
                'Q': (Path.CURVE3,)*2,
                'C': (Path.CURVE4,)*3,
                'Z': (Path.CLOSEPOLY,)}
    vertices = []
    codes = []
    cmd_values = re.split("([A-Za-z])", path)[1:]
    for cmd, values in zip(cmd_values[::2], cmd_values[1::2]):
        points = ([*map(float, re.split(",|(?<!^)(?=[+-])", values))] if values
                  else [(0., 0.)])
        points = np.reshape(points, (-1, 2))
        if cmd.islower():
            points += vertices[-1][-1]
        codes.extend(commands[cmd.upper()])
        vertices.append(points)
    return np.array(codes), np.concatenate(vertices)


@case(f"{G}/firefox", figsize=(5, 5))
def _(fig):
    codes, verts = _svg_parse(FIREFOX)
    path = Path(verts, codes)
    xmin, ymin = verts.min(axis=0) - 1
    xmax, ymax = verts.max(axis=0) + 1
    fig.set_facecolor("0.75")
    ax = fig.add_axes((0, 0, 1, 1), frameon=False, aspect=1, xlim=(xmin, xmax),
                      ylim=(ymax, ymin), xticks=[], yticks=[])
    ax.add_patch(PathPatch(path, facecolor='none', edgecolor='w', lw=6))
    ax.add_patch(PathPatch(path, facecolor='orange', edgecolor='k', lw=2))


# --- integral -----------------------------------------------------------------


@case(f"{G}/integral")
def _(fig):
    def func(x):
        return (x - 3) * (x - 5) * (x - 7) + 85

    a, b = 2, 9
    x = np.linspace(0, 10)
    ax = fig.subplots()
    ax.plot(x, func(x), 'r', linewidth=2)
    ax.set_ylim(bottom=0)
    ix = np.linspace(a, b)
    verts = [(a, 0), *zip(ix, func(ix)), (b, 0)]
    ax.add_patch(Polygon(verts, facecolor='0.9', edgecolor='0.5'))
    ax.text(0.5 * (a + b), 30, r"$\int_a^b f(x)\mathrm{d}x$", horizontalalignment='center',
            fontsize=20)
    fig.text(0.9, 0.05, '$x$')
    fig.text(0.1, 0.9, '$y$')
    ax.spines[['top', 'right']].set_visible(False)
    ax.set_xticks([a, b], labels=['$a$', '$b$'])
    ax.set_yticks([])


# --- mandelbrot ---------------------------------------------------------------


def _mandelbrot_set(xmin, xmax, ymin, ymax, xn, yn, maxiter, horizon=2.0):
    X = np.linspace(xmin, xmax, xn).astype(np.float32)
    Y = np.linspace(ymin, ymax, yn).astype(np.float32)
    C = X + Y[:, None] * 1j
    N = np.zeros_like(C, dtype=int)
    Z = np.zeros_like(C)
    for n in range(maxiter):
        I = abs(Z) < horizon  # noqa: E741
        N[I] = n
        Z[I] = Z[I]**2 + C[I]
    N[N == maxiter-1] = 0
    return Z, N


@case(f"{G}/mandelbrot", figsize=(10, 10 * 1250 / 1500), dpi=72)
def _(fig):
    xmin, xmax, xn = -2.25, +0.75, 3000 // 2
    ymin, ymax, yn = -1.25, +1.25, 2500 // 2
    maxiter = 200
    horizon = 2.0 ** 40
    log_horizon = np.log2(np.log(horizon))
    Z, N = _mandelbrot_set(xmin, xmax, ymin, ymax, xn, yn, maxiter, horizon)
    with np.errstate(invalid='ignore'):
        M = np.nan_to_num(N + 1 - np.log2(np.log(abs(Z))) + log_horizon)
    ax = fig.add_axes((0, 0, 1, 1), frameon=False, aspect=1)
    light = mcolors.LightSource(azdeg=315, altdeg=10)
    M = light.shade(M, cmap=plt.colormaps["hot"], vert_exag=1.5, norm=mcolors.PowerNorm(0.3),
                    blend_mode='hsv')
    ax.imshow(M, extent=[xmin, xmax, ymin, ymax], interpolation="bicubic")
    ax.set_xticks([])
    ax.set_yticks([])
    ax.text(xmin+.025, ymin+.025, "The Mandelbrot fractal set\n"
            "Rendered with matplotlib - https://matplotlib.org",
            color="white", fontsize=12, alpha=0.5)


# --- pan_zoom_overlap ---------------------------------------------------------


@case(f"{G}/pan_zoom_overlap", figsize=(11, 6))
def _(fig):
    fig.suptitle("Showcase for pan/zoom events on overlapping axes.")
    ax = fig.add_axes((.05, .05, .9, .9))
    ax.patch.set_color(".75")
    ax.twinx()
    ax1 = fig.add_subplot(221)
    ax1.twinx()
    ax1.text(.5, .5, "Visible patch\n\nPan/zoom events are NOT\nforwarded to axes below",
             ha="center", va="center", transform=ax1.transAxes)
    ax11 = fig.add_subplot(223, sharex=ax1, sharey=ax1)
    ax11.set_forward_navigation_events(True)
    ax11.text(.5, .5, "Visible patch\n\nOverride capture behavior:\n\n"
              "ax.set_forward_navigation_events(True)",
              ha="center", va="center", transform=ax11.transAxes)
    ax2 = fig.add_subplot(222)
    ax2.twinx()
    ax2.patch.set_visible(False)
    ax2.text(.5, .5, "Invisible patch\n\nPan/zoom events are\nforwarded to axes below",
             ha="center", va="center", transform=ax2.transAxes)
    ax22 = fig.add_subplot(224, sharex=ax2, sharey=ax2)
    ax22.patch.set_visible(False)
    ax22.set_forward_navigation_events(False)
    ax22.text(.5, .5, "Invisible patch\n\nOverride capture behavior:\n\n"
              "ax.set_forward_navigation_events(False)",
              ha="center", va="center", transform=ax22.transAxes)


# --- stock_prices -------------------------------------------------------------

#: The tickers of ``stock_prices`` with, for their synthetic prices, the
#: first month quoted, the first price and the mean monthly log return.
STOCKS = {
    'IBM': (0, 11.0, 0.0066), 'AAPL': (0, 0.24, 0.0167), 'MSFT': (0, 0.4, 0.0167),
    'XRX': (0, 11.2, 0.001), 'AMZN': (89, 0.08, 0.0245), 'DELL': (320, 13.3, 0.015),
    'GOOGL': (176, 65.0, 0.0175), 'ADBE': (0, 1.4, 0.016), 'GSPC': (0, 330.0, 0.0065),
    'IXIC': (0, 416.0, 0.0087),
}


def _stock_data():
    """Monthly prices from January 1990 to June 2022 in place of
    ``Stocks.csv``: a geometric random walk per ticker, NaN before its
    first month."""
    dates = np.arange("1990-01", "2022-07", dtype="datetime64[M]").astype("datetime64[D]")
    g = rng(SEED)
    prices = {}
    for ticker, (first, start, drift) in STOCKS.items():
        steps = g.normal(len(dates), loc=drift, scale=0.07)
        steps[:first + 1] = 0
        walk = start * np.exp(np.cumsum(steps))
        walk[:first] = np.nan
        prices[ticker] = walk
    return dates, prices


@case(f"{G}/stock_prices", figsize=(6, 8))
def _(fig):
    dates, stock_data = _stock_data()
    fig.set_layout_engine("constrained")
    ax = fig.subplots(1, 1)
    ax.set_prop_cycle(color=[
        '#1f77b4', '#aec7e8', '#ff7f0e', '#ffbb78', '#2ca02c', '#98df8a',
        '#d62728', '#ff9896', '#9467bd', '#c5b0d5', '#8c564b', '#c49c94',
        '#e377c2', '#f7b6d2', '#7f7f7f', '#c7c7c7', '#bcbd22', '#dbdb8d',
        '#17becf', '#9edae5'])
    stocks_name = ['IBM', 'Apple', 'Microsoft', 'Xerox', 'Amazon', 'Dell', 'Alphabet', 'Adobe',
                   'S&P 500', 'NASDAQ']
    y_offsets = dict.fromkeys(STOCKS, 0)
    y_offsets['IBM'] = 5
    y_offsets['AAPL'] = -5
    y_offsets['AMZN'] = -6
    for nn, column in enumerate(STOCKS):
        good = np.nonzero(np.isfinite(stock_data[column]))
        line, = ax.plot(dates[good], stock_data[column][good], lw=2.5)
        y_pos = stock_data[column][-1]
        offset = y_offsets[column] / 72
        trans = ax.transData + mtransforms.ScaledTranslation(0, offset, fig.dpi_scale_trans)
        ax.text(np.datetime64('2022-10-01'), y_pos, stocks_name[nn], color=line.get_color(),
                transform=trans)
    ax.set_xlim(np.datetime64('1989-06-01'), np.datetime64('2023-01-01'))
    fig.suptitle("Technology company stocks prices dollars (1990-2022)", ha="center")
    ax.spines[:].set_visible(False)
    ax.xaxis.tick_bottom()
    ax.yaxis.tick_left()
    ax.set_yscale('log')
    ax.grid(True, 'major', 'both', ls='--', lw=.5, c='k', alpha=.3)
    ax.tick_params(axis='both', which='both', labelsize='large', bottom=False, top=False,
                   labelbottom=True, left=False, right=False, labelleft=True)


# --- xkcd ---------------------------------------------------------------------


def _xkcd_rc():
    """The rcParams ``plt.xkcd()`` changes, as a case ``rc``."""
    before = dict(mpl.rcParams)
    with plt.xkcd():
        return {key: value for key, value in mpl.rcParams.items() if value != before[key]}


@case(f"{G}/xkcd_2", rc=_xkcd_rc())
def _(fig):
    ax = fig.add_axes((0.1, 0.2, 0.8, 0.7))
    ax.bar([0, 1], [0, 100], 0.25)
    ax.spines[['top', 'right']].set_visible(False)
    ax.xaxis.set_ticks_position('bottom')
    ax.set_xticks([0, 1])
    ax.set_xticklabels(['CONFIRMED BY\nEXPERIMENT', 'REFUTED BY\nEXPERIMENT'])
    ax.set_xlim(-0.5, 1.5)
    ax.set_yticks([])
    ax.set_ylim(0, 110)
    ax.set_title("CLAIMS OF SUPERNATURAL POWERS")
    fig.text(0.5, 0.05, '"The Data So Far" from xkcd by Randall Munroe', ha='center')


# === spines ===================================================================


def _sine_wave():
    x = np.linspace(0, 2 * np.pi, 100)
    return x, 2 * np.sin(x)


@case(f"{G}/spine_placement_demo")
def _(fig):
    x, y = _sine_wave()
    ax_dict = fig.subplot_mosaic([['center', 'zero'], ['axes', 'data']])
    fig.suptitle('Spine positions')
    for name, title, position in [('center', "'center'", 'center'),
                                  ('zero', "'zero'", 'zero')]:
        ax = ax_dict[name]
        ax.set_title(title)
        ax.plot(x, y)
        ax.spines[['left', 'bottom']].set_position(position)
        ax.spines[['top', 'right']].set_visible(False)
    ax = ax_dict['axes']
    ax.set_title("'axes' (0.2, 0.2)")
    ax.plot(x, y)
    ax.spines.left.set_position(('axes', 0.2))
    ax.spines.bottom.set_position(('axes', 0.2))
    ax.spines[['top', 'right']].set_visible(False)
    ax = ax_dict['data']
    ax.set_title("'data' (1, 2)")
    ax.plot(x, y)
    ax.spines.left.set_position(('data', 1))
    ax.spines.bottom.set_position(('data', 2))
    ax.spines[['top', 'right']].set_visible(False)


@case(f"{G}/spines")
def _(fig):
    x, y = _sine_wave()
    fig.set_layout_engine("constrained")
    ax0, ax1, ax2 = fig.subplots(nrows=3)
    ax0.plot(x, y)
    ax0.set_title('normal spines')
    ax1.plot(x, y)
    ax1.set_title('bottom-left spines')
    ax1.spines.right.set_visible(False)
    ax1.spines.top.set_visible(False)
    ax2.plot(x, y)
    ax2.set_title('spines with bounds limited to data range')
    ax2.spines.bottom.set_bounds(x.min(), x.max())
    ax2.spines.left.set_bounds(y.min(), y.max())
    ax2.spines.right.set_visible(False)
    ax2.spines.top.set_visible(False)


# === pyplots ==================================================================


@case(f"{G}/pyplot_simple")
def _(fig):
    x = np.arange(0.0, 2.0, 0.01)
    ax = fig.subplots()
    ax.plot(x, np.sin(2 * np.pi * x))
    ax.set_title("A basic plot using pyplot")
    ax.set_xlabel('Time [s]')
    ax.set_ylabel('Voltage [mV]')


@case(f"{G}/pyplot_two_subplots")
def _(fig):
    def f(t):
        return np.exp(-t) * np.cos(2*np.pi*t)

    t1 = np.arange(0.0, 5.0, 0.1)
    t2 = np.arange(0.0, 5.0, 0.02)
    ax = fig.add_subplot(211)
    ax.plot(t1, f(t1), color='tab:blue', marker='o')
    ax.plot(t2, f(t2), color='black')
    ax.set_title("Subplot 1")
    ax = fig.add_subplot(212)
    ax.plot(t2, np.cos(2*np.pi*t2), color='tab:orange', linestyle='--')
    ax.set_title("Subplot 2")
