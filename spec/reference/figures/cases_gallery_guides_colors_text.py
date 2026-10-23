"""matplotlib's user guide pages on colours and text, its quick start guide
and its rcParams guide (``galleries/users_explain``: ``colors/*``,
``text/*``, ``quick_start``, ``customizing``), one case per figure:
``<script>_<n>`` for the script's ``n``-th figure.

The scripts run as written, with these substitutions: ``np.random`` data
comes from ``spec_support.rng``; ``topobathy.npz`` is replaced by a
synthetic coast on the same grid (``_topobathy``); the CAM02-UCS lightness
``colormaps`` plots comes from a port of colorspacious' formulas
(``_lightness``), colorspacious not being part of the reference
environment. rcParams and styles a script sets stay in force for its later
figures, as they do when the script runs (``colors``, ``colormaps``,
``customizing``); colormaps and box styles a script registers are
unregistered once its case is built.

Not ported: the font fallback figure of ``fonts`` (it needs Noto Sans CJK),
the ``pgf`` and ``usetex`` pages (LaTeX) and ``configuration`` (no
figure). Figures other cases already draw are left out: ``annotations_4``
(``gallery_text/annotation_polar``), ``text_intro_1``
(``gallery_text/text_commands``) and ``quick_start_4`` (the pyplot version
of ``quick_start_3``)."""

import datetime
from functools import partial

import matplotlib as mpl
import matplotlib.dates as mdates
import matplotlib.style
import matplotlib.colorizer
import numpy as np
from cycler import cycler
from matplotlib import colors as mcolors
from matplotlib import ticker
from matplotlib.colors import LinearSegmentedColormap, ListedColormap
from matplotlib.dates import ConciseDateFormatter
from matplotlib.font_manager import FontProperties
from matplotlib.offsetbox import (AnchoredOffsetbox, AnchoredText, AnnotationBbox, DrawingArea,
                                  HPacker, OffsetImage, TextArea)
from matplotlib.patches import (Annulus, BoxStyle, Circle, ConnectionPatch, Ellipse,
                                FancyArrowPatch, Rectangle)
from matplotlib.path import Path
from matplotlib.text import OffsetFrom
from mpl_toolkits.axes_grid1.anchored_artists import AnchoredAuxTransformBox, AnchoredDrawingArea

from spec_support import case, rng
from spec_support.gallery_data import SEED

G = "gallery_guides_colors_text"


# --- colors/colorbar_only -----------------------------------------------------


def _standalone(fig, norm, cmap, **colorbar):
    """``fig.colorbar`` of a colorizing artist that holds no data."""
    colorizer = mpl.colorizer.Colorizer(norm=norm, cmap=cmap)
    fig.colorbar(mpl.colorizer.ColorizingArtist(colorizer), **colorbar)


def _strip(fig, norm, cmap, **colorbar):
    """The tutorial's usual figure: a horizontal standalone colorbar drawn
    on the only axes of a constrained figure."""
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    _standalone(fig, norm, cmap, cax=ax, orientation='horizontal', **colorbar)


@case(f"{G}/colorbar_only_1", figsize=(6, 1))
def _(fig):
    _strip(fig, mcolors.Normalize(vmin=5, vmax=10), "cool", label='Some Units')


@case(f"{G}/colorbar_only_2")
def _(fig):
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    _standalone(fig, mcolors.Normalize(0, 1), 'magma', ax=ax, orientation='vertical',
                label='a colorbar label')


@case(f"{G}/colorbar_only_3", figsize=(6, 1))
def _(fig):
    cmap = mpl.colormaps["viridis"]
    bounds = [-1, 2, 5, 7, 12, 15]
    norm = mcolors.BoundaryNorm(bounds, cmap.N, extend='both')
    _strip(fig, norm, 'viridis', label="Discrete intervals with extend='both' keyword")


@case(f"{G}/colorbar_only_4", figsize=(6, 1))
def _(fig):
    cmap = ListedColormap(['red', 'green', 'blue', 'cyan'], under='yellow', over='magenta')
    norm = mcolors.BoundaryNorm([1, 2, 4, 7, 8], cmap.N)
    _strip(fig, norm, cmap, extend='both', spacing='proportional',
           label='Discrete intervals, some other units')


@case(f"{G}/colorbar_only_5", figsize=(6, 1))
def _(fig):
    cmap = ListedColormap(['royalblue', 'cyan', 'yellow', 'orange'], over='red', under='blue')
    norm = mcolors.BoundaryNorm([-1.0, -0.5, 0.0, 0.5, 1.0], cmap.N)
    _strip(fig, norm, cmap, extend='both', extendfrac='auto', spacing='uniform',
           label='Custom extension lengths, some other units')


# --- colors/colormap-manipulation ---------------------------------------------


def _plot_examples(fig, colormaps):
    """``plot_examples``: the same random field under each colormap, each
    with a colorbar, on a figure ``2 * len(colormaps) + 2`` by 3 in."""
    data = rng(SEED).normal(900).reshape(30, 30)
    fig.set_layout_engine('constrained')
    axs = fig.subplots(1, len(colormaps), squeeze=False)
    for ax, cmap in zip(axs.flat, colormaps):
        psm = ax.pcolormesh(data, cmap=cmap, rasterized=True, vmin=-4, vmax=4)
        fig.colorbar(psm, ax=ax)


#: The colours of the tutorial's first colormaps.
_ORANGE_TO_TEAL = ["darkorange", "gold", "lawngreen", "lightseagreen"]


def _viridis():
    return mpl.colormaps['viridis'].resampled(256)


@case(f"{G}/colormap_manipulation_1", figsize=(4, 3))
def _(fig):
    _plot_examples(fig, [ListedColormap(_ORANGE_TO_TEAL)])


@case(f"{G}/colormap_manipulation_2", figsize=(6, 3))
def _(fig):
    viridis = _viridis()
    newcolors = viridis(np.linspace(0, 1, 256))
    pink = np.array([248/256, 24/256, 148/256, 1])
    newcolors[:25, :] = pink
    _plot_examples(fig, [viridis, ListedColormap(newcolors)])


@case(f"{G}/colormap_manipulation_3", figsize=(6, 3))
def _(fig):
    viridis_big = mpl.colormaps['viridis']
    newcmp = ListedColormap(viridis_big(np.linspace(0.25, 0.75, 128)))
    _plot_examples(fig, [_viridis(), newcmp])


@case(f"{G}/colormap_manipulation_4", figsize=(6, 3))
def _(fig):
    top = mpl.colormaps['Oranges_r'].resampled(128)
    bottom = mpl.colormaps['Blues'].resampled(128)
    newcolors = np.vstack((top(np.linspace(0, 1, 128)), bottom(np.linspace(0, 1, 128))))
    _plot_examples(fig, [_viridis(), ListedColormap(newcolors, name='OrangeBlue')])


@case(f"{G}/colormap_manipulation_5", figsize=(6, 3))
def _(fig):
    N = 256
    vals = np.ones((N, 4))
    vals[:, 0] = np.linspace(90/256, 1, N)
    vals[:, 1] = np.linspace(40/256, 1, N)
    vals[:, 2] = np.linspace(40/256, 1, N)
    _plot_examples(fig, [_viridis(), ListedColormap(vals)])


_CDICT = {'red': [[0.0, 0.0, 0.0],
                  [0.5, 1.0, 1.0],
                  [1.0, 1.0, 1.0]],
          'green': [[0.0, 0.0, 0.0],
                    [0.25, 0.0, 0.0],
                    [0.75, 1.0, 1.0],
                    [1.0, 1.0, 1.0]],
          'blue': [[0.0, 0.0, 0.0],
                   [0.5, 0.0, 0.0],
                   [1.0, 1.0, 1.0]]}


def _plot_linearmap(fig, cdict):
    """``plot_linearmap``: the red, green and blue channels of the
    segmented colormap ``cdict`` describes."""
    newcmp = LinearSegmentedColormap('testCmap', segmentdata=cdict, N=256)
    rgba = newcmp(np.linspace(0, 1, 256))
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    for xx in [0.25, 0.5, 0.75]:
        ax.axvline(xx, color='0.7', linestyle='--')
    for i, color in enumerate(['r', 'g', 'b']):
        ax.plot(np.arange(256)/256, rgba[:, i], color=color)
    ax.set_xlabel('index')
    ax.set_ylabel('RGB')


@case(f"{G}/colormap_manipulation_6", figsize=(4, 3))
def _(fig):
    _plot_linearmap(fig, _CDICT)


@case(f"{G}/colormap_manipulation_7", figsize=(4, 3))
def _(fig):
    _plot_linearmap(fig, {**_CDICT, 'red': [[0.0, 0.0, 0.3],
                                            [0.5, 1.0, 0.9],
                                            [1.0, 1.0, 1.0]]})


@case(f"{G}/colormap_manipulation_8", figsize=(6, 3))
def _(fig):
    cmap1 = LinearSegmentedColormap.from_list("mycmap", _ORANGE_TO_TEAL)
    nodes = [0.0, 0.4, 0.8, 1.0]
    cmap2 = LinearSegmentedColormap.from_list("mycmap", list(zip(nodes, _ORANGE_TO_TEAL)))
    _plot_examples(fig, [cmap1, cmap2])


def _my_cmap():
    return ListedColormap(["#ffffcc", "#a1dab4", "#41b6c4", "#2c7fb8", "#253494"],
                          name="my_cmap")


@case(f"{G}/colormap_manipulation_9", figsize=(6, 3))
def _(fig):
    my_cmap = _my_cmap()
    _plot_examples(fig, [my_cmap, my_cmap.reversed()])


@case(f"{G}/colormap_manipulation_10")
def _(fig):
    my_cmap = _my_cmap()
    mpl.colormaps.register(cmap=my_cmap)
    mpl.colormaps.register(cmap=my_cmap.reversed())
    try:
        data = [[1, 2, 3, 4, 5]]
        ax1, ax2 = fig.subplots(nrows=2)
        ax1.imshow(data, cmap='my_cmap')
        ax2.imshow(data, cmap='my_cmap_r')
    finally:
        mpl.colormaps.unregister('my_cmap')
        mpl.colormaps.unregister('my_cmap_r')


# --- colors/colormapnorms -----------------------------------------------------


def _rows(fig, draw, mappings, extend):
    """One row per colour mapping (keyword arguments of ``draw``), each
    with a colorbar extended as ``extend`` says; returns the axes."""
    axs = fig.subplots(len(mappings), 1)
    for ax, mapping in zip(axs, mappings):
        fig.colorbar(draw(ax, **mapping), ax=ax, extend=extend)
    return axs


@case(f"{G}/colormapnorms_1")
def _(fig):
    N = 100
    X, Y = np.mgrid[-3:3:complex(0, N), -2:2:complex(0, N)]
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X * 10)**2 - (Y * 10)**2)
    Z = Z1 + 50 * Z2
    _rows(fig, lambda ax, **kw: ax.pcolor(X, Y, Z, cmap='PuBu_r', shading='auto', **kw),
          [dict(norm=mcolors.LogNorm(vmin=Z.min(), vmax=Z.max())), {}], 'max')


@case(f"{G}/colormapnorms_2")
def _(fig):
    delta = 0.1
    x = np.arange(-3.0, 4.001, delta)
    y = np.arange(-4.0, 3.001, delta)
    X, Y = np.meshgrid(x, y)
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X - 1)**2 - (Y - 1)**2)
    Z = (0.9*Z1 - 0.5*Z2) * 2
    ax1, ax2 = fig.subplots(ncols=2)
    for ax, norm, title in [(ax1, None, 'Normalize()'),
                            (ax2, mcolors.CenteredNorm(), 'CenteredNorm()')]:
        pc = ax.pcolormesh(Z, norm=norm, cmap="coolwarm")
        fig.colorbar(pc, ax=ax)
        ax.set_title(title)


@case(f"{G}/colormapnorms_3")
def _(fig):
    N = 100
    X, Y = np.mgrid[-3:3:complex(0, N), -2:2:complex(0, N)]
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X - 1)**2 - (Y - 1)**2)
    Z = (Z1 - Z2) * 2
    symlog = mcolors.SymLogNorm(linthresh=0.03, linscale=0.03, vmin=-1.0, vmax=1.0, base=10)
    _rows(fig, lambda ax, **kw: ax.pcolormesh(X, Y, Z, cmap='RdBu_r', shading='auto', **kw),
          [dict(norm=symlog), dict(vmin=-np.max(Z))], 'both')


def _power_field():
    """The power-law sections' field: stripes that grow with ``x``."""
    N = 100
    X, Y = np.mgrid[0:3:complex(0, N), 0:2:complex(0, N)]
    return X, Y, (1 + np.sin(Y * 10.)) * X**2


@case(f"{G}/colormapnorms_4")
def _(fig):
    X, Y, Z1 = _power_field()
    fig.set_layout_engine('constrained')
    ax = _rows(fig, lambda ax, **kw: ax.pcolormesh(X, Y, Z1, cmap='PuBu_r', shading='auto', **kw),
               [dict(norm=mcolors.PowerNorm(gamma=0.5)), {}], 'max')
    ax[0].set_title('PowerNorm()')
    ax[1].set_title('Normalize()')


@case(f"{G}/colormapnorms_5", figsize=(8, 6))
def _(fig):
    N = 100
    X, Y = np.meshgrid(np.linspace(-3, 3, N), np.linspace(-2, 2, N))
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X - 1)**2 - (Y - 1)**2)
    Z = ((Z1 - Z2) * 2)[:-1, :-1]
    fig.set_layout_engine('constrained')
    ax = fig.subplots(2, 2).flatten()
    pcm = ax[0].pcolormesh(X, Y, Z, cmap='RdBu_r')
    fig.colorbar(pcm, ax=ax[0], orientation='vertical')
    ax[0].set_title('Default norm')
    even = np.linspace(-1.5, 1.5, 7)
    for axis, norm, extend, title in [
        (ax[1], mcolors.BoundaryNorm(boundaries=even, ncolors=256), 'both',
         'BoundaryNorm: 7 boundaries'),
        (ax[2], mcolors.BoundaryNorm(boundaries=np.array([-0.2, -0.1, 0, 0.5, 1]), ncolors=256),
         'both', 'BoundaryNorm: nonuniform'),
        (ax[3], mcolors.BoundaryNorm(boundaries=even, ncolors=256, extend='both'), None,
         'BoundaryNorm: extend="both"'),
    ]:
        pcm = axis.pcolormesh(X, Y, Z, norm=norm, cmap='RdBu_r')
        fig.colorbar(pcm, ax=axis, extend=extend, orientation='vertical')
        axis.set_title(title)


def _topobathy():
    """A synthetic stand-in for ``topobathy.npz`` on its grid (120
    longitudes from 234.02 to 237.98, 91 latitudes from 48.02 to 49.98):
    sea floor sloping down to about -900 m west of a wavy coast, hills up
    to about 2300 m east of it."""
    longitude = 234.0 + (np.arange(120) + 0.5) / 30
    latitude = np.linspace(48.0164, 49.9842, 91)
    lon, lat = np.meshgrid(longitude, latitude)
    shore = lon - (235.7 + 0.3 * np.sin(4 * lat))
    ridges = 250 * np.sin(5 * lon) * np.cos(7 * lat)
    noise = rng(SEED).normal(91 * 120, scale=40.0).reshape(91, 120)
    return longitude, latitude, np.where(shore < 0, 400, 900) * shore + ridges + noise


def _terrain_map():
    """The tutorial's land and sea colormap: 256 colours of the blue start
    of ``terrain`` for the sea, 256 of its rest for the land."""
    colors_undersea = mpl.colormaps["terrain"](np.linspace(0, 0.17, 256))
    colors_land = mpl.colormaps["terrain"](np.linspace(0.25, 1, 256))
    all_colors = np.vstack((colors_undersea, colors_land))
    return LinearSegmentedColormap.from_list('terrain_map', all_colors)


def _topo_map(ax, norm, title):
    """The tutorial's map of the coast under ``norm``, its aspect set for
    latitude 49°; returns the mesh."""
    longitude, latitude, topo = _topobathy()
    pcm = ax.pcolormesh(longitude, latitude, topo, rasterized=True, norm=norm,
                        cmap=_terrain_map(), shading='auto')
    ax.set_aspect(1 / np.cos(np.deg2rad(49)))
    ax.set_title(title)
    return pcm


_TOPO_TICKS = [-500, 0, 1000, 2000, 3000, 4000]


@case(f"{G}/colormapnorms_6")
def _(fig):
    ax = fig.subplots()
    divnorm = mcolors.TwoSlopeNorm(vmin=-500., vcenter=0, vmax=4000)
    pcm = _topo_map(ax, divnorm, 'TwoSlopeNorm(x)')
    cb = fig.colorbar(pcm, shrink=0.6)
    cb.set_ticks(_TOPO_TICKS)


@case(f"{G}/colormapnorms_7", figsize=(10, 4))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    divnorm = mcolors.TwoSlopeNorm(vmin=-500., vcenter=0, vmax=4000)
    for ax, title in zip([ax1, ax2], ['Default: Scaled colorbar', 'Linear colorbar spacing']):
        pcm = _topo_map(ax, divnorm, title)
        cb = fig.colorbar(pcm, ax=ax, shrink=0.6)
        cb.set_ticks(np.arange(-500, 4001, 500))
    cb.ax.set_yscale('linear')


def _forward(x):
    return np.sqrt(x)


def _inverse(x):
    return x**2


@case(f"{G}/colormapnorms_8")
def _(fig):
    X, Y, Z1 = _power_field()
    ax = fig.subplots()
    norm = mcolors.FuncNorm((_forward, _inverse), vmin=0, vmax=20)
    pcm = ax.pcolormesh(X, Y, Z1, norm=norm, cmap='PuBu_r', shading='auto')
    ax.set_title('FuncNorm(x)')
    fig.colorbar(pcm, shrink=0.6)


class MidpointNormalize(mcolors.Normalize):
    """The tutorial's custom norm: linear from ``vmin`` to ``vcenter`` and
    from there to ``vmax``, extrapolated to infinities."""

    def __init__(self, vmin=None, vmax=None, vcenter=None, clip=False):
        self.vcenter = vcenter
        super().__init__(vmin, vmax, clip)

    def __call__(self, value, clip=None):
        x, y = [self.vmin, self.vcenter, self.vmax], [0, 0.5, 1.]
        return np.ma.masked_array(np.interp(value, x, y, left=-np.inf, right=np.inf))

    def inverse(self, value):
        y, x = [self.vmin, self.vcenter, self.vmax], [0, 0.5, 1]
        return np.interp(value, x, y, left=-np.inf, right=np.inf)


@case(f"{G}/colormapnorms_9")
def _(fig):
    ax = fig.subplots()
    midnorm = MidpointNormalize(vmin=-500., vcenter=0, vmax=4000)
    pcm = _topo_map(ax, midnorm, 'Custom norm')
    cb = fig.colorbar(pcm, shrink=0.6, extend='both')
    cb.set_ticks(_TOPO_TICKS)


# --- colors/colormaps ---------------------------------------------------------


_CMAPS = {
    'Perceptually Uniform Sequential': ['viridis', 'plasma', 'inferno', 'magma', 'cividis'],
    'Sequential': ['Greys', 'Purples', 'Blues', 'Greens', 'Oranges', 'Reds',
                   'YlOrBr', 'YlOrRd', 'OrRd', 'PuRd', 'RdPu', 'BuPu',
                   'GnBu', 'PuBu', 'YlGnBu', 'PuBuGn', 'BuGn', 'YlGn'],
    'Sequential (2)': ['gray', 'bone', 'pink', 'spring', 'summer', 'autumn',
                       'winter', 'cool', 'Wistia', 'hot', 'afmhot', 'gist_heat',
                       'copper'],
    'Diverging': ['PiYG', 'PRGn', 'BrBG', 'PuOr', 'RdGy', 'RdBu', 'RdYlBu',
                  'RdYlGn', 'Spectral', 'coolwarm', 'bwr', 'seismic',
                  'berlin', 'managua', 'vanimo'],
    'Cyclic': ['twilight', 'twilight_shifted', 'hsv'],
    'Qualitative': ['Pastel1', 'Pastel2', 'Paired', 'Accent', 'okabe_ito',
                    'Dark2', 'Set1', 'Set2', 'Set3', 'tab10', 'tab20',
                    'tab20b', 'tab20c'],
    'Miscellaneous': ['flag', 'prism', 'ocean', 'gist_earth', 'terrain',
                      'gist_stern', 'gnuplot', 'gnuplot2', 'CMRmap',
                      'cubehelix', 'brg', 'gist_rainbow', 'rainbow', 'jet',
                      'turbo', 'nipy_spectral', 'gist_ncar'],
}

_GRADIENT = np.vstack((np.linspace(0, 1, 256), np.linspace(0, 1, 256)))


def _gradients_height(nrows):
    """The height of the gradients figure of ``nrows`` colormaps."""
    return 0.35 + 0.15 + (nrows + (nrows - 1) * 0.1) * 0.22


def _plot_color_gradients(fig, category):
    """The first ``plot_color_gradients``: a strip of each colormap of the
    category, named on its left."""
    cmap_list = _CMAPS[category]
    nrows = len(cmap_list)
    figh = _gradients_height(nrows)
    axs = fig.subplots(nrows=nrows + 1)
    fig.subplots_adjust(top=1 - 0.35 / figh, bottom=0.15 / figh, left=0.2, right=0.99)
    axs[0].set_title(f'{category} colormaps', fontsize=14)
    for ax, name in zip(axs, cmap_list):
        ax.imshow(_GRADIENT, aspect='auto', cmap=mpl.colormaps[name])
        ax.text(-0.01, 0.5, name, va='center', ha='right', fontsize=10, transform=ax.transAxes)
    for ax in axs:
        ax.set_axis_off()


#: colorspacious' constants for sRGB under sRGB viewing conditions: the
#: sRGB to XYZ (in percent) matrix, the CAT02 and Hunt-Pointer-Estevez cone
#: matrices and the D65 white point.
_SRGB_TO_XYZ100 = 100 * np.array([[0.4124, 0.3576, 0.1805],
                                  [0.2126, 0.7152, 0.0722],
                                  [0.0193, 0.1192, 0.9505]])
_CAT02 = np.array([[0.7328, 0.4296, -0.1624],
                   [-0.7036, 1.6975, 0.0061],
                   [0.0030, 0.0136, 0.9834]])
_HPE = np.array([[0.38971, 0.68898, -0.07868],
                 [-0.22981, 1.18340, 0.04641],
                 [0.00000, 0.00000, 1.00000]])
_D65 = np.array([95.047, 100.0, 108.883])


def _lightness(rgb):
    """The CAM02-UCS lightness J' of the sRGB colours ``rgb`` (n x 3, in
    [0, 1]): colorspacious' ``cspace_converter("sRGB1", "CAM02-UCS")(rgb)``
    restricted to its first channel, for a 64 lux surround (adapting
    luminance ``64 / pi / 5``), background luminance 20 and an average
    surround. A negative achromatic response (pure black, through rounding)
    counts as zero."""
    L_A, Y_b, F, c = 64 / np.pi / 5, 20.0, 1.0, 0.69
    k = 1 / (5 * L_A + 1)
    F_L = 0.2 * k**4 * (5 * L_A) + 0.1 * (1 - k**4)**2 * (5 * L_A)**(1 / 3)
    D = F * (1 - (1 / 3.6) * np.exp((-L_A - 42) / 92))
    D_RGB = D * _D65[1] / (_CAT02 @ _D65) + 1 - D
    n = Y_b / _D65[1]
    z = 1.48 + np.sqrt(n)
    N_bb = 0.725 * (1 / n)**0.2
    to_cones = _HPE @ np.linalg.inv(_CAT02)

    def achromatic(xyz):
        cones = (D_RGB * (xyz @ _CAT02.T)) @ to_cones.T
        t = (F_L * np.abs(cones) / 100)**0.42
        adapted = np.sign(cones) * 400 * (t / (t + 27.13)) + 0.1
        return np.maximum((adapted @ np.array([2, 1, 1 / 20]) - 0.305) * N_bb, 0)

    linear = np.where(rgb < 0.04045, rgb / 12.92, ((rgb + 0.055) / 1.055)**2.4)
    J = 100 * (achromatic(linear @ _SRGB_TO_XYZ100.T) / achromatic(_D65))**(c * z)
    return 1.7 * J / (1 + 0.007 * J)


#: Colormaps per lightness subplot, and their horizontal spacing.
_DSUBS = {'Perceptually Uniform Sequential': 5, 'Sequential': 6,
          'Sequential (2)': 6, 'Diverging': 6, 'Cyclic': 3,
          'Qualitative': 4, 'Miscellaneous': 6}
_DC = {'Perceptually Uniform Sequential': 1.4, 'Sequential': 0.7,
       'Sequential (2)': 1.4, 'Diverging': 1.4, 'Cyclic': 1.4,
       'Qualitative': 1.4, 'Miscellaneous': 1.4}

_X = np.linspace(0.0, 1.0, 100)


def _lightness_subplots(category):
    return int(np.ceil(len(_CMAPS[category]) / _DSUBS[category]))


def _plot_lightness(fig, category):
    """The lightness figure of a category: the lightness of each colormap
    as a band of its colours, a few colormaps per subplot."""
    cmap_list = _CMAPS[category]
    dsub = _DSUBS[category]
    dc = _DC[category]
    axs = fig.subplots(nrows=_lightness_subplots(category), squeeze=False)
    for i, ax in enumerate(axs.flat):
        locs = []
        for j, cmap in enumerate(cmap_list[i*dsub:(i+1)*dsub]):
            lightness = _lightness(mpl.colormaps[cmap](_X)[:, :3])
            if category == 'Sequential':
                y_, c_ = lightness[::-1], _X[::-1]
            else:
                y_, c_ = lightness, _X
            ax.scatter(_X + j*dc, y_, c=c_, cmap=cmap, s=300, linewidths=0.0)
            if category in ('Perceptually Uniform Sequential', 'Sequential'):
                locs.append(_X[-1] + j*dc)
            else:
                locs.append(_X[int(_X.size/2.)] + j*dc)
        ax.set_xlim(axs[0, 0].get_xlim())
        ax.set_ylim(0.0, 100.0)
        ax.xaxis.set_ticks_position('top')
        ax.xaxis.set_major_locator(ticker.FixedLocator(locs))
        ax.xaxis.set_major_formatter(ticker.FixedFormatter(cmap_list[i*dsub:(i+1)*dsub]))
        ax.xaxis.set_tick_params(rotation=50)
        ax.set_ylabel('Lightness $L^*$', fontsize=12)
    ax.set_xlabel(category + ' colormaps', fontsize=14)
    fig.tight_layout(h_pad=0.0, pad=1.5)


def _plot_grayscale(fig, category):
    """The second ``plot_color_gradients``: each colormap of the category
    beside the grey of its lightness."""
    cmap_list = _CMAPS[category]
    axs = fig.subplots(nrows=len(cmap_list), ncols=2)
    fig.subplots_adjust(top=0.95, bottom=0.01, left=0.2, right=0.99, wspace=0.05)
    fig.suptitle(category + ' colormaps', fontsize=14, y=1.0, x=0.6)
    for ax, name in zip(axs, cmap_list):
        L = _lightness(mpl.colormaps[name](_X)[:, :3])
        L = np.float32(np.vstack((L, L, L)))
        ax[0].imshow(_GRADIENT, aspect='auto', cmap=mpl.colormaps[name])
        ax[1].imshow(L, aspect='auto', cmap='binary_r', vmin=0., vmax=100.)
        pos = list(ax[0].get_position().bounds)
        x_text = pos[0] - 0.01
        y_text = pos[1] + pos[3]/2.
        fig.text(x_text, y_text, name, va='center', ha='right', fontsize=10)
    for ax in axs.flat:
        ax.set_axis_off()


for _i, _category in enumerate(_CMAPS, start=1):
    case(f"{G}/colormaps_{_i}", figsize=(6.4, _gradients_height(len(_CMAPS[_category]))))(
        partial(_plot_color_gradients, category=_category))
    case(f"{G}/colormaps_{_i + 7}", figsize=(7, 2.6 * _lightness_subplots(_category)),
         rc={'font.size': 12})(partial(_plot_lightness, category=_category))
    case(f"{G}/colormaps_{_i + 14}", rc={'font.size': 14})(
        partial(_plot_grayscale, category=_category))


# --- colors/colors ------------------------------------------------------------


@case(f"{G}/colors_1", figsize=(6.5, 1.65))
def _(fig):
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.add_patch(Rectangle((-0.2, -0.35), 11.2, 0.7, color='C1', alpha=0.8))
    for i, alpha in enumerate(np.linspace(0, 1, 11)):
        ax.add_patch(Rectangle((i, 0.05), 0.8, 0.6, alpha=alpha, zorder=0))
        ax.text(i+0.4, 0.85, f"{alpha:.1f}", ha='center')
        ax.add_patch(Rectangle((i, -0.05), 0.8, -0.6, alpha=alpha, zorder=2))
    ax.set_xlim(-0.2, 13)
    ax.set_ylim(-1, 1)
    ax.set_title('alpha values')
    ax.text(11.3, 0.6, 'zorder=1', va='center', color='C0')
    ax.text(11.3, 0, 'zorder=2\nalpha=0.8', va='center', color='C1')
    ax.text(11.3, -0.6, 'zorder=3', va='center', color='C0')
    ax.axis('off')


def _style_demo(fig, sty):
    """``demo``: the second and third colours of the style's cycle, the
    title in its first."""
    th = np.linspace(0, 2*np.pi, 128)
    ax = fig.subplots()
    ax.set_title(f'style: {sty!r}', color='C0')
    ax.plot(th, np.cos(th), 'C1', label='C1')
    ax.plot(th, np.sin(th), 'C2', label='C2')
    ax.legend()


#: The style ``colors`` switches to for its third figure, which stays in
#: force for the fourth, with the font family pinned as in ``cases_styles``
#: (the sheet prefers Arial and Liberation Sans, which hosts may or may not
#: have).
_SEABORN = {**mpl.style.library['seaborn-v0_8'], 'font.family': 'DejaVu Sans'}


@case(f"{G}/colors_2", figsize=(3, 3))
def _(fig):
    _style_demo(fig, 'default')


@case(f"{G}/colors_3", figsize=(3, 3), rc=_SEABORN)
def _(fig):
    _style_demo(fig, 'seaborn-v0_8')


@case(f"{G}/colors_4", figsize=(9, 5), rc=_SEABORN)
def _(fig):
    overlap = {name for name in mcolors.CSS4_COLORS
               if f'xkcd:{name}' in mcolors.XKCD_COLORS}
    ax = fig.add_axes((0, 0, 1, 1))
    n_groups = 3
    n_rows = len(overlap) // n_groups + 1
    for j, color_name in enumerate(sorted(overlap)):
        css4 = mcolors.CSS4_COLORS[color_name]
        xkcd = mcolors.XKCD_COLORS[f'xkcd:{color_name}'].upper()
        rgba = mcolors.to_rgba_array([css4, xkcd])
        luma = 0.299 * rgba[:, 0] + 0.587 * rgba[:, 1] + 0.114 * rgba[:, 2]
        css4_text_color = 'k' if luma[0] > 0.5 else 'w'
        xkcd_text_color = 'k' if luma[1] > 0.5 else 'w'
        col_shift = (j // n_rows) * 3
        y_pos = j % n_rows
        text_args = dict(fontsize=10, weight='bold' if css4 == xkcd else None)
        ax.add_patch(Rectangle((0 + col_shift, y_pos), 1, 1, color=css4))
        ax.add_patch(Rectangle((1 + col_shift, y_pos), 1, 1, color=xkcd))
        ax.text(0.5 + col_shift, y_pos + .7, css4, color=css4_text_color, ha='center',
                **text_args)
        ax.text(1.5 + col_shift, y_pos + .7, xkcd, color=xkcd_text_color, ha='center',
                **text_args)
        ax.text(2 + col_shift, y_pos + .7, f'  {color_name}', **text_args)
    for g in range(n_groups):
        ax.hlines(range(n_rows), 3*g, 3*g + 2.8, color='0.7', linewidth=1)
        ax.text(0.5 + 3*g, -0.3, 'X11/CSS4', ha='center')
        ax.text(1.5 + 3*g, -0.3, 'xkcd', ha='center')
    ax.set_xlim(0, 3 * n_groups)
    ax.set_ylim(n_rows, -1)
    ax.axis('off')


# --- text/annotations ---------------------------------------------------------


def _cosine(ax):
    """The guides' line: ``cos 2πt`` over ``[0, 5)``, two points wide."""
    t = np.arange(0.0, 5.0, 0.01)
    s = np.cos(2*np.pi*t)
    ax.plot(t, s, lw=2)


def _local_max(fig):
    """The guides' first annotation: an arrow from text to a maximum."""
    ax = fig.subplots()
    _cosine(ax)
    ax.annotate('local max', xy=(2, 1), xytext=(3, 1.5),
                arrowprops=dict(facecolor='black', shrink=0.05))
    ax.set_ylim(-2, 2)


@case(f"{G}/annotations_1", figsize=(3, 3))
def _(fig):
    _local_max(fig)


@case(f"{G}/annotations_2", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    _cosine(ax)
    ax.annotate('local max', xy=(2, 1), xycoords='data', xytext=(0.01, .99),
                textcoords='axes fraction', va='top', ha='left',
                arrowprops=dict(facecolor='black', shrink=0.05))
    ax.set_ylim(-2, 2)


@case(f"{G}/annotations_3", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    arr = FancyArrowPatch((1.25, 1.5), (1.75, 1.5), arrowstyle='->,head_width=.15',
                          mutation_scale=20)
    ax.add_patch(arr)
    ax.annotate("label", (.5, .5), xycoords=arr, ha='center', va='bottom')
    ax.set(xlim=(1, 2), ylim=(1, 2))


@case(f"{G}/annotations_5", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    x = [1, 3, 5, 7, 9]
    y = [2, 4, 6, 8, 10]
    annotations = ["A", "B", "C", "D", "E"]
    ax.scatter(x, y, s=20)
    for xi, yi, text in zip(x, y, annotations):
        ax.annotate(text, xy=(xi, yi), xycoords='data', xytext=(1.5, 1.5),
                    textcoords='offset points')


@case(f"{G}/annotations_6", figsize=(5, 5))
def _(fig):
    ax = fig.subplots()
    ax.text(0.5, 0.5, "Direction", ha="center", va="center", rotation=45, size=15,
            bbox=dict(boxstyle="rarrow,pad=0.3", fc="lightblue", ec="steelblue", lw=2))


def _angled_box(x0, y0, width, height, pad):
    """The tutorial's custom box: the text box padded by ``pad`` with a
    point on its left side."""
    width = width + 2 * pad
    height = height + 2 * pad
    x0, y0 = x0 - pad, y0 - pad
    x1, y1 = x0 + width, y0 + height
    return Path([(x0, y0), (x1, y0), (x1, y1), (x0, y1),
                 (x0-pad, (y0+y1)/2), (x0, y0), (x0, y0)],
                closed=True)


def custom_box_style(x0, y0, width, height, mutation_size):
    """The tutorial's box style function, padding by 0.3 of the font size."""
    return _angled_box(x0, y0, width, height, mutation_size * 0.3)


class MyStyle:
    """The tutorial's box style class, padding by ``pad`` of the font size."""

    def __init__(self, pad=0.3):
        self.pad = pad
        super().__init__()

    def __call__(self, x0, y0, width, height, mutation_size):
        return _angled_box(x0, y0, width, height, mutation_size * self.pad)


def _test_box(fig, boxstyle):
    ax = fig.subplots()
    ax.text(0.5, 0.5, "Test", size=30, va="center", ha="center", rotation=30,
            bbox=dict(boxstyle=boxstyle, alpha=0.2))


@case(f"{G}/annotations_7", figsize=(3, 3))
def _(fig):
    _test_box(fig, custom_box_style)


@case(f"{G}/annotations_8", figsize=(3, 3))
def _(fig):
    BoxStyle._style_list["angled"] = MyStyle
    try:
        _test_box(fig, "angled,pad=0.5")
    finally:
        del BoxStyle._style_list["angled"]


@case(f"{G}/annotations_9", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ax.annotate("", xy=(0.2, 0.2), xycoords='data', xytext=(0.8, 0.8), textcoords='data',
                arrowprops=dict(arrowstyle="->", connectionstyle="arc3"))


@case(f"{G}/annotations_10", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ax.annotate("Test", xy=(0.2, 0.2), xycoords='data', xytext=(0.8, 0.8), textcoords='data',
                size=20, va="center", ha="center",
                arrowprops=dict(arrowstyle="simple", connectionstyle="arc3,rad=-0.2"))


def _round4_test(ax, connectionstyle, **arrow):
    """An annotation in a round4 box with a white ``-|>`` arrow."""
    ax.annotate("Test", xy=(0.2, 0.2), xycoords='data', xytext=(0.8, 0.8), textcoords='data',
                size=20, va="center", ha="center", bbox=dict(boxstyle="round4", fc="w"),
                arrowprops=dict(arrowstyle="-|>", connectionstyle=connectionstyle, fc="w",
                                **arrow))


@case(f"{G}/annotations_11", figsize=(3, 3))
def _(fig):
    _round4_test(fig.subplots(), "arc3,rad=-0.2")


@case(f"{G}/annotations_12", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    _round4_test(ax, "arc3,rad=0.2", relpos=(0., 0.))
    _round4_test(ax, "arc3,rad=-0.2", relpos=(1., 0.))


@case(f"{G}/annotations_13", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    at = AnchoredText("Figure 1a", prop=dict(size=15), frameon=True, loc='upper left')
    at.patch.set_boxstyle("round,pad=0.,rounding_size=0.2")
    ax.add_artist(at)


@case(f"{G}/annotations_14", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ada = AnchoredDrawingArea(40, 20, 0, 0, loc='upper right', pad=0., frameon=False)
    p1 = Circle((10, 10), 10)
    ada.drawing_area.add_artist(p1)
    p2 = Circle((30, 10), 5, fc="r")
    ada.drawing_area.add_artist(p2)
    ax.add_artist(ada)


@case(f"{G}/annotations_15", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    box = AnchoredAuxTransformBox(ax.transData, loc='upper left')
    el = Ellipse((0, 0), width=0.1, height=0.4, angle=30)
    box.drawing_area.add_artist(el)
    ax.add_artist(box)


@case(f"{G}/annotations_16", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    box1 = TextArea(" Test: ", textprops=dict(color="k"))
    box2 = DrawingArea(60, 20, 0, 0)
    el1 = Ellipse((10, 10), width=16, height=5, angle=30, fc="r")
    el2 = Ellipse((30, 10), width=16, height=5, angle=170, fc="g")
    el3 = Ellipse((50, 10), width=16, height=5, angle=230, fc="b")
    box2.add_artist(el1)
    box2.add_artist(el2)
    box2.add_artist(el3)
    box = HPacker(children=[box1, box2], align="center", pad=0, sep=5)
    anchored_box = AnchoredOffsetbox(loc='lower left', child=box, pad=0., frameon=True,
                                     bbox_to_anchor=(0., 1.02), bbox_transform=ax.transAxes,
                                     borderpad=0.)
    ax.add_artist(anchored_box)
    fig.subplots_adjust(top=0.8)


@case(f"{G}/annotations_17")
def _(fig):
    ax = fig.subplots()
    text = ax.text(.2, .8, "Green!", color='green')
    da = DrawingArea(20, 20)
    annulus = Annulus((10, 10), 10, 5, color='tab:green')
    da.add_artist(annulus)
    ab1 = AnnotationBbox(da, xy=(.5, 0), xybox=(.5, .25), xycoords=text,
                         boxcoords=(text, "data"), arrowprops=dict(arrowstyle="->"),
                         bboxprops=dict(alpha=0.5))
    ax.add_artist(ab1)
    N = 25
    arr = np.repeat(np.linspace(0, 1, N), N).reshape(N, N)
    im = OffsetImage(arr, cmap='Greens')
    im.image.axes = ax
    ab2 = AnnotationBbox(im, xy=(.5, 0), xybox=(.75, 0), xycoords=text,
                         boxcoords=('data', annulus), arrowprops=dict(arrowstyle="->"),
                         bboxprops=dict(alpha=0.5))
    ax.add_artist(ab2)


@case(f"{G}/annotations_18", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(nrows=1, ncols=2)
    ax1.annotate("Test", xy=(0.2, 0.2), xycoords=ax1.transAxes)
    ax2.annotate("Test", xy=(0.2, 0.2), xycoords="axes fraction")


@case(f"{G}/annotations_19", figsize=(6, 3))
def _(fig):
    x = np.linspace(-1, 1)
    ax1, ax2 = fig.subplots(nrows=1, ncols=2)
    ax1.plot(x, -x**3)
    ax2.plot(x, -3*x**2)
    ax2.annotate("", xy=(0, 0), xycoords=ax1.transData, xytext=(0, 0),
                 textcoords=ax2.transData, arrowprops=dict(arrowstyle="<->"))


def _test_1(ax):
    """The tutorial's "Test 1" annotation, centred in a round box."""
    return ax.annotate("Test 1", xy=(0.5, 0.5), xycoords="data", va="center", ha="center",
                       bbox=dict(boxstyle="round", fc="w"))


def _test_2_beside(fig, coords):
    """"Test 2" 30 points right of "Test 1", its point at ``(1, 0.5)`` of
    what ``coords`` gives for "Test 1"."""
    ax = fig.subplots()
    an1 = _test_1(ax)
    ax.annotate("Test 2", xy=(1, 0.5), xycoords=coords(an1), xytext=(30, 0),
                textcoords="offset points", va="center", ha="left",
                bbox=dict(boxstyle="round", fc="w"), arrowprops=dict(arrowstyle="->"))


@case(f"{G}/annotations_20", figsize=(3, 3))
def _(fig):
    _test_2_beside(fig, lambda an1: an1)


@case(f"{G}/annotations_21", figsize=(3, 3))
def _(fig):
    _test_2_beside(fig, lambda an1: an1.get_window_extent)


@case(f"{G}/annotations_22", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(nrows=1, ncols=2)
    ax1.annotate("Test1", xy=(0.5, 0.5), xycoords="axes fraction")
    ax2.annotate("Test 2", xy=(0.5, 0.5), xycoords=ax2.get_window_extent)


@case(f"{G}/annotations_23", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    ax.annotate("Test", xy=(0.5, 1), xycoords=("data", "axes fraction"))
    ax.axvline(x=.5, color='lightgray')
    ax.set(xlim=(0, 2), ylim=(1, 2))


@case(f"{G}/annotations_24", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    t1 = ax.text(0.05, .05, "Text 1", va='bottom', ha='left')
    t2 = ax.text(0.90, .90, "Text 2", ha='right')
    ax.annotate("Anchored to 1 & 2", xy=(0, 0), xycoords=(t1, t2), va='bottom',
                color='tab:orange')


@case(f"{G}/annotations_25", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    an1 = _test_1(ax)
    offset_from = OffsetFrom(an1, (0.5, 0))
    ax.annotate("Test 2", xy=(0.1, 0.1), xycoords="data", xytext=(0, -10),
                textcoords=offset_from, va="top", ha="center",
                bbox=dict(boxstyle="round", fc="w"), arrowprops=dict(arrowstyle="->"))


@case(f"{G}/annotations_26", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(nrows=1, ncols=2)
    xy = (0.3, 0.2)
    con = ConnectionPatch(xyA=xy, coordsA=ax1.transData, xyB=xy, coordsB=ax2.transData)
    fig.add_artist(con)


# --- text/mathtext ------------------------------------------------------------


def _framed(fig):
    """``plt.figure(figsize=(3, 3), linewidth=1, edgecolor='black')``."""
    fig.set_linewidth(1)
    fig.set_edgecolor('black')


@case(f"{G}/mathtext_1", figsize=(3, 3))
def _(fig):
    _framed(fig)
    fig.text(.2, .7, "plain text: alpha > beta")
    fig.text(.2, .5, "Mathtext: $\\alpha > \\beta$")
    fig.text(.2, .3, r"raw string Mathtext: $\alpha > \beta$")


@case(f"{G}/mathtext_2", figsize=(3, 3))
def _(fig):
    _framed(fig)
    fig.suptitle("Number of unescaped $")
    fig.text(.1, .7, r"odd: $ \alpha $ = $1")
    fig.text(.1, .5, r"even: $ \beta $= $ 2 $")
    fig.text(.1, .3, r'odd: $ \gamma $= \$3 $')
    fig.text(.1, .1, r'even: $ \delta $ = $ \$4 $')


# --- text/text_props ----------------------------------------------------------


@case(f"{G}/text_props_1")
def _(fig):
    left, width = 0.25, 0.5
    bottom, height = 0.25, 0.5
    right = left + width
    top = bottom + height
    ax = fig.add_axes((0, 0, 1, 1))
    p = Rectangle((left, bottom), width, height, fill=False, transform=ax.transAxes,
                  clip_on=False)
    ax.add_patch(p)
    middle = 0.5*(bottom+top)
    vertical = dict(rotation='vertical')
    for x, y, text, ha, va, style in [
        (left, bottom, 'left top', 'left', 'top', {}),
        (left, bottom, 'left bottom', 'left', 'bottom', {}),
        (right, top, 'right bottom', 'right', 'bottom', {}),
        (right, top, 'right top', 'right', 'top', {}),
        (right, bottom, 'center top', 'center', 'top', {}),
        (left, middle, 'right center', 'right', 'center', vertical),
        (left, middle, 'left center', 'left', 'center', vertical),
        (0.5*(left+right), middle, 'middle', 'center', 'center', dict(fontsize=20, color='red')),
        (right, middle, 'centered', 'center', 'center', vertical),
        (left, top, 'rotated\nwith newlines', 'center', 'center', dict(rotation=45)),
    ]:
        ax.text(x, y, text, horizontalalignment=ha, verticalalignment=va, transform=ax.transAxes,
                **style)
    ax.set_axis_off()


# --- text/text_intro ----------------------------------------------------------


_X1 = np.linspace(0.0, 5.0, 100)
_Y1 = np.cos(2 * np.pi * _X1) * np.exp(-_X1)


def _oscillation(fig, scale=1.0):
    """The tutorial's damped oscillation (times ``scale``) on a 5 x 3 in
    figure with room for the labels; returns the axes."""
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.15, left=0.2)
    ax.plot(_X1, _Y1 * scale)
    return ax


@case(f"{G}/text_intro_2", figsize=(5, 3))
def _(fig):
    ax = _oscillation(fig)
    ax.set_xlabel('Time (s)')
    ax.set_ylabel('Damped oscillation (V)')


@case(f"{G}/text_intro_3", figsize=(5, 3))
def _(fig):
    ax = _oscillation(fig, 10000)
    ax.set_xlabel('Time (s)')
    ax.set_ylabel('Damped oscillation (V)')


@case(f"{G}/text_intro_4", figsize=(5, 3))
def _(fig):
    ax = _oscillation(fig, 10000)
    ax.set_xlabel('Time (s)')
    ax.set_ylabel('Damped oscillation (V)', labelpad=18)


@case(f"{G}/text_intro_5", figsize=(5, 3))
def _(fig):
    ax = _oscillation(fig)
    ax.set_xlabel('Time (s)', position=(0., 1e6), horizontalalignment='left')
    ax.set_ylabel('Damped oscillation (V)')


@case(f"{G}/text_intro_6", figsize=(5, 3))
def _(fig):
    font = FontProperties(family='Times New Roman', style='italic')
    ax = _oscillation(fig)
    ax.set_xlabel('Time (s)', fontsize='large', fontweight='bold')
    ax.set_ylabel('Damped oscillation (V)', fontproperties=font)


@case(f"{G}/text_intro_7", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(bottom=0.2, left=0.2)
    ax.plot(_X1, np.cumsum(_Y1**2))
    ax.set_xlabel('Time (s) \n This was a long experiment')
    ax.set_ylabel(r'$\int\ Y^2\ dt\ \ (V^2 s)$')


@case(f"{G}/text_intro_8", figsize=(5, 6))
def _(fig):
    fig.set_layout_engine('tight')
    axs = fig.subplots(3, 1)
    for ax, loc in zip(axs, ['center', 'left', 'right']):
        ax.plot(_X1, _Y1)
        ax.set_title('Title with loc at ' + loc, loc=loc)


@case(f"{G}/text_intro_9", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(top=0.8)
    ax.plot(_X1, _Y1)
    ax.set_title('Vertically offset title', pad=30)


#: The ticks the tutorial sets: ``np.arange(0., 8.1, 2.)``.
_TICKS = np.arange(0., 8.1, 2.)


def _oscillation_pair(fig):
    """Two tight rows of the damped oscillation; returns their axes."""
    fig.set_layout_engine('tight')
    axs = fig.subplots(2, 1)
    axs[0].plot(_X1, _Y1)
    axs[1].plot(_X1, _Y1)
    return axs


@case(f"{G}/text_intro_10", figsize=(5, 3))
def _(fig):
    axs = _oscillation_pair(fig)
    axs[1].xaxis.set_ticks(_TICKS)


@case(f"{G}/text_intro_11", figsize=(5, 3))
def _(fig):
    axs = _oscillation_pair(fig)
    tickla = [f'{tick:1.2f}' for tick in _TICKS]
    axs[1].xaxis.set_ticks(_TICKS)
    axs[1].xaxis.set_ticklabels(tickla)
    axs[1].set_xlim(axs[0].get_xlim())


@case(f"{G}/text_intro_12", figsize=(5, 3))
def _(fig):
    axs = _oscillation_pair(fig)
    axs[1].xaxis.set_ticks(_TICKS)
    axs[1].xaxis.set_major_formatter('{x:1.1f}')
    axs[1].set_xlim(axs[0].get_xlim())


@case(f"{G}/text_intro_13", figsize=(5, 3))
def _(fig):
    axs = _oscillation_pair(fig)
    axs[1].xaxis.set_major_locator(ticker.FixedLocator(_TICKS))
    axs[1].xaxis.set_major_formatter('±{x}°')


@case(f"{G}/text_intro_14", figsize=(8, 5))
def _(fig):
    fig.set_layout_engine('tight')
    axs = fig.subplots(2, 2)
    for ax in axs.flat:
        ax.plot(_X1*10., _Y1)
    axs[0, 1].xaxis.set_major_locator(ticker.MaxNLocator(nbins='auto', steps=[1, 4, 10]))
    axs[0, 1].xaxis.set_major_formatter(ticker.FormatStrFormatter('%1.1f'))
    axs[1, 0].xaxis.set_major_formatter(ticker.FormatStrFormatter('%1.5f'))
    axs[1, 0].xaxis.set_major_locator(ticker.AutoLocator())
    axs[1, 1].xaxis.set_major_formatter(ticker.FormatStrFormatter('%1.5f'))
    axs[1, 1].xaxis.set_major_locator(ticker.MaxNLocator(nbins=4))


def formatoddticks(x, pos):
    """Format odd tick positions."""
    if x % 2:
        return f'{x:1.2f}'
    else:
        return ''


@case(f"{G}/text_intro_15", figsize=(5, 3))
def _(fig):
    fig.set_layout_engine('tight')
    ax = fig.subplots()
    ax.plot(_X1, _Y1)
    ax.xaxis.set_major_formatter(formatoddticks)
    ax.xaxis.set_major_locator(ticker.MaxNLocator(nbins=6))


def _daily(fig):
    """Tight axes for the damped oscillation over 100 days of 2017; returns
    them and the days."""
    base = datetime.datetime(2017, 1, 1, 0, 0, 1)
    time = [base + datetime.timedelta(days=x) for x in range(len(_X1))]
    fig.set_layout_engine('tight')
    ax = fig.subplots()
    return ax, time


@case(f"{G}/text_intro_16", figsize=(5, 3))
def _(fig):
    ax, time = _daily(fig)
    ax.plot(time, _Y1)
    ax.tick_params(axis='x', rotation=70)


@case(f"{G}/text_intro_17", figsize=(5, 3))
def _(fig):
    ax, time = _daily(fig)
    ax.xaxis.set_major_locator(mdates.DayLocator(bymonthday=[1, 15]))
    ax.xaxis.set_major_formatter(mdates.DateFormatter('%b %d'))
    ax.plot(time, _Y1)
    ax.tick_params(axis='x', rotation=70)


# --- quick_start --------------------------------------------------------------


def _random_sets():
    """``data1, data2, data3, data4 = np.random.randn(4, 100)``."""
    return rng(SEED).normal(400).reshape(4, 100)


#: The guide's small figure size.
_SMALL = (5, 2.7)


@case(f"{G}/quick_start_1")
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3, 4], [1, 4, 2, 3])


@case(f"{G}/quick_start_2", figsize=_SMALL)
def _(fig):
    g = rng(SEED)
    data = {'a': np.arange(50), 'c': g.integers(50, 0, 50), 'd': g.normal(50)}
    data['b'] = data['a'] + 10 * g.normal(50)
    data['d'] = np.abs(data['d']) * 100
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.scatter('a', 'b', c='c', s='d', data=data)
    ax.set_xlabel('entry a')
    ax.set_ylabel('entry b')


@case(f"{G}/quick_start_3", figsize=_SMALL)
def _(fig):
    x = np.linspace(0, 2, 100)
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.plot(x, x, label='linear')
    ax.plot(x, x**2, label='quadratic')
    ax.plot(x, x**3, label='cubic')
    ax.set_xlabel('x label')
    ax.set_ylabel('y label')
    ax.set_title("Simple Plot")
    ax.legend()


def my_plotter(ax, data1, data2, param_dict):
    """A helper function to make a graph."""
    return ax.plot(data1, data2, **param_dict)


@case(f"{G}/quick_start_5", figsize=_SMALL)
def _(fig):
    data1, data2, data3, data4 = _random_sets()
    ax1, ax2 = fig.subplots(1, 2)
    my_plotter(ax1, data1, data2, {'marker': 'x'})
    my_plotter(ax2, data3, data4, {'marker': 'o'})


@case(f"{G}/quick_start_6", figsize=_SMALL)
def _(fig):
    data1, data2, _, _ = _random_sets()
    ax = fig.subplots()
    x = np.arange(len(data1))
    ax.plot(x, np.cumsum(data1), color='blue', linewidth=3, linestyle='--')
    l, = ax.plot(x, np.cumsum(data2), color='orange', linewidth=2)
    l.set_linestyle(':')


@case(f"{G}/quick_start_7", figsize=_SMALL)
def _(fig):
    data1, data2, _, _ = _random_sets()
    ax = fig.subplots()
    ax.scatter(data1, data2, s=50, facecolor='C0', edgecolor='k')


@case(f"{G}/quick_start_8", figsize=_SMALL)
def _(fig):
    data1, data2, data3, data4 = _random_sets()
    ax = fig.subplots()
    ax.plot(data1, 'o', label='data1')
    ax.plot(data2, 'd', label='data2')
    ax.plot(data3, 'v', label='data3')
    ax.plot(data4, 's', label='data4')
    ax.legend()


@case(f"{G}/quick_start_9", figsize=_SMALL)
def _(fig):
    mu, sigma = 115, 15
    x = mu + sigma * rng(SEED).normal(10000)
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.hist(x, 50, density=True, facecolor='C0', alpha=0.75)
    ax.set_xlabel('Length [cm]')
    ax.set_ylabel('Probability')
    ax.set_title('Aardvark lengths\n (not really)')
    ax.text(75, .025, r'$\mu=115,\ \sigma=15$')
    ax.axis([55, 175, 0, 0.03])
    ax.grid(True)


@case(f"{G}/quick_start_10", figsize=_SMALL)
def _(fig):
    _local_max(fig)


@case(f"{G}/quick_start_11", figsize=_SMALL)
def _(fig):
    data1, data2, data3, _ = _random_sets()
    ax = fig.subplots()
    ax.plot(np.arange(len(data1)), data1, label='data1')
    ax.plot(np.arange(len(data2)), data2, label='data2')
    ax.plot(np.arange(len(data3)), data3, 'd', label='data3')
    ax.legend()


@case(f"{G}/quick_start_12", figsize=_SMALL)
def _(fig):
    data1 = _random_sets()[0]
    fig.set_layout_engine('constrained')
    axs = fig.subplots(1, 2)
    xdata = np.arange(len(data1))
    data = 10**data1
    axs[0].plot(xdata, data)
    axs[1].set_yscale('log')
    axs[1].plot(xdata, data)


@case(f"{G}/quick_start_13")
def _(fig):
    data1 = _random_sets()[0]
    xdata = np.arange(len(data1))
    fig.set_layout_engine('constrained')
    axs = fig.subplots(2, 1)
    axs[0].plot(xdata, data1)
    axs[0].set_title('Automatic ticks')
    axs[1].plot(xdata, data1)
    axs[1].set_xticks(np.arange(0, 100, 30), ['zero', '30', 'sixty', '90'])
    axs[1].set_yticks([-1.5, 0, 1.5])
    axs[1].set_title('Manual ticks')


@case(f"{G}/quick_start_14", figsize=_SMALL)
def _(fig):
    dates = np.arange(np.datetime64('2021-11-15'), np.datetime64('2021-12-25'),
                      np.timedelta64(1, 'h'))
    data = np.cumsum(rng(SEED).normal(len(dates)))
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.plot(dates, data)
    ax.xaxis.set_major_formatter(ConciseDateFormatter(ax.xaxis.get_major_locator()))


@case(f"{G}/quick_start_15", figsize=_SMALL)
def _(fig):
    categories = ['turnips', 'rutabaga', 'cucumber', 'pumpkins']
    fig.set_layout_engine('constrained')
    ax = fig.subplots()
    ax.bar(categories, rng(SEED).uniform(len(categories)))


@case(f"{G}/quick_start_16", figsize=(7, 2.7))
def _(fig):
    t = np.arange(0.0, 5.0, 0.01)
    s = np.cos(2 * np.pi * t)
    fig.set_layout_engine('constrained')
    ax1, ax3 = fig.subplots(1, 2)
    l1, = ax1.plot(t, s)
    ax2 = ax1.twinx()
    l2, = ax2.plot(t, range(len(t)), 'C1')
    ax2.legend([l1, l2], ['Sine (left)', 'Straight (right)'])
    ax3.plot(t, s)
    ax3.set_xlabel('Angle [rad]')
    ax4 = ax3.secondary_xaxis('top', (np.rad2deg, np.deg2rad))
    ax4.set_xlabel('Angle [°]')


@case(f"{G}/quick_start_17")
def _(fig):
    data1, data2, data3, _ = _random_sets()
    X, Y = np.meshgrid(np.linspace(-3, 3, 128), np.linspace(-3, 3, 128))
    Z = (1 - X/2 + X**5 + Y**3) * np.exp(-X**2 - Y**2)
    fig.set_layout_engine('constrained')
    axs = fig.subplots(2, 2)
    pc = axs[0, 0].pcolormesh(X, Y, Z, vmin=-1, vmax=1, cmap='RdBu_r')
    fig.colorbar(pc, ax=axs[0, 0])
    axs[0, 0].set_title('pcolormesh()')
    co = axs[0, 1].contourf(X, Y, Z, levels=np.linspace(-1.25, 1.25, 11))
    fig.colorbar(co, ax=axs[0, 1])
    axs[0, 1].set_title('contourf()')
    pc = axs[1, 0].imshow(Z**2 * 100, cmap='plasma', norm=mcolors.LogNorm(vmin=0.01, vmax=100))
    fig.colorbar(pc, ax=axs[1, 0], extend='both')
    axs[1, 0].set_title('imshow() with LogNorm()')
    pc = axs[1, 1].scatter(data1, data2, c=data3, cmap='RdBu_r')
    fig.colorbar(pc, ax=axs[1, 1], extend='both')
    axs[1, 1].set_title('scatter()')


@case(f"{G}/quick_start_18")
def _(fig):
    fig.set_layout_engine('constrained')
    axd = fig.subplot_mosaic([['upleft', 'right'],
                              ['lowleft', 'right']])
    axd['upleft'].set_title('upleft')
    axd['lowleft'].set_title('lowleft')
    axd['right'].set_title('right')


# --- customizing --------------------------------------------------------------


#: The rcParams ``customizing`` sets, in the order it sets them.
_DASHED = {'lines.linewidth': 2, 'lines.linestyle': '--'}
_RGBY = {**_DASHED, 'axes.prop_cycle': cycler(color=['r', 'g', 'b', 'y'])}
_DASHDOT = {**_RGBY, 'lines.linewidth': 4, 'lines.linestyle': '-.'}


def _plot_data(fig):
    """``plt.plot(data)`` for the guide's 50 random values."""
    fig.subplots().plot(rng(SEED).normal(50))


case(f"{G}/customizing_1", rc=_DASHED)(_plot_data)
case(f"{G}/customizing_2", rc=_RGBY)(_plot_data)
case(f"{G}/customizing_3", rc=_DASHDOT)(_plot_data)
case(f"{G}/customizing_4", rc={**_DASHDOT, 'lines.linewidth': 2, 'lines.linestyle': ':'})(
    _plot_data)
case(f"{G}/customizing_5", rc={**_DASHDOT, 'lines.linewidth': 3, 'lines.linestyle': '-'})(
    _plot_data)


@case(f"{G}/customizing_6", rc={**_DASHDOT, **mpl.style.library['ggplot'],
                                **mpl.style.library['dark_background']})
def _(fig):
    fig.subplots().plot(np.sin(np.linspace(0, 2 * np.pi)), 'r-o')
