"""matplotlib's gallery section "Images, contours and fields": images
(interpolation, transparency, placement, clipping, norms), quadrilateral
and triangular meshes, line and filled contours with labels and hatches,
quiver and barbs, streamlines and colormap normalisations.

Each case ports one figure of an example (``<example>_<n>`` when the
example draws several). Random data comes from ``spec_support.rng`` seeded
with the example's seed; sample data files are replaced by synthetic arrays
of the same shape (see ``_photo``, ``_scan``, ``_logo`` and ``_terrain``).
``affine_image`` is covered by ``images/affine_transforms``.
"""

import math

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import colors, patheffects, ticker, tri
from matplotlib.image import BboxImage, NonUniformImage
from matplotlib.patches import Circle, PathPatch, Rectangle
from matplotlib.path import Path
from matplotlib.transforms import Bbox, TransformedBbox

from spec_support import case, rng

SEED = 19680801


# --- shared data -----------------------------------------------------------

def _two_bumps(X, Y, exp=np.exp):
    """The gallery's favourite field: a positive and a negative bump."""
    Z1 = exp(-X**2 - Y**2)
    Z2 = exp(-(X - 1)**2 - (Y - 1)**2)
    return (Z1 - Z2) * 2


#: libm's ``exp``, value by value. ``numpy.exp`` takes a SIMD path that
#: depends on the CPU and differs from libm in the last bit of some values.
_libm_exp = np.vectorize(math.exp)


def _contour_field():
    """``contour_demo``'s grid: ``_two_bumps`` over [-3, 3) x [-2, 2).

    Its zero contour is the straight line x + y = 1, where every label
    position is about as good as any other: which one ``clabel`` picks
    depends on the last bits of the values, so they come from libm's
    ``exp``, which gives the same bits everywhere."""
    X, Y = np.meshgrid(np.arange(-3.0, 3.0, 0.025), np.arange(-2.0, 2.0, 0.025))
    return X, Y, _two_bumps(X, Y, exp=_libm_exp)


def _spike(X, Y):
    """A wide bump with a narrow spike fifty times higher on top."""
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X * 10)**2 - (Y * 10)**2)
    return Z1 + 50 * Z2


def _func3(x, y):
    return (1 - x / 2 + x**5 + y**3) * np.exp(-(x**2 + y**2))


def _annulus(n_angles, n_radii, min_radius):
    """Points on staggered rings between ``min_radius`` and 0.95, their
    radii and angles."""
    radii = np.linspace(min_radius, 0.95, n_radii)
    angles = np.linspace(0, 2 * np.pi, n_angles, endpoint=False)
    angles = np.repeat(angles[..., np.newaxis], n_radii, axis=1)
    angles[:, 1::2] += np.pi / n_angles
    x = (radii * np.cos(angles)).flatten()
    y = (radii * np.sin(angles)).flatten()
    return x, y, radii, angles


def _masked_delaunay(x, y, min_radius):
    """The Delaunay triangulation of ``x, y`` without the triangles inside
    ``min_radius``."""
    triang = tri.Triangulation(x, y)
    triang.set_mask(np.hypot(x[triang.triangles].mean(axis=1),
                             y[triang.triangles].mean(axis=1)) < min_radius)
    return triang


#: The outline of Great Britain (radians) of the tri examples.
BRITAIN = np.asarray([
    [-0.101, 0.872], [-0.080, 0.883], [-0.069, 0.888], [-0.054, 0.890],
    [-0.045, 0.897], [-0.057, 0.895], [-0.073, 0.900], [-0.087, 0.898],
    [-0.090, 0.904], [-0.069, 0.907], [-0.069, 0.921], [-0.080, 0.919],
    [-0.073, 0.928], [-0.052, 0.930], [-0.048, 0.942], [-0.062, 0.949],
    [-0.054, 0.958], [-0.069, 0.954], [-0.087, 0.952], [-0.087, 0.959],
    [-0.080, 0.966], [-0.085, 0.973], [-0.087, 0.965], [-0.097, 0.965],
    [-0.097, 0.975], [-0.092, 0.984], [-0.101, 0.980], [-0.108, 0.980],
    [-0.104, 0.987], [-0.102, 0.993], [-0.115, 1.001], [-0.099, 0.996],
    [-0.101, 1.007], [-0.090, 1.010], [-0.087, 1.021], [-0.069, 1.021],
    [-0.052, 1.022], [-0.052, 1.017], [-0.069, 1.010], [-0.064, 1.005],
    [-0.048, 1.005], [-0.031, 1.005], [-0.031, 0.996], [-0.040, 0.987],
    [-0.045, 0.980], [-0.052, 0.975], [-0.040, 0.973], [-0.026, 0.968],
    [-0.020, 0.954], [-0.006, 0.947], [0.003, 0.935], [0.006, 0.926],
    [0.005, 0.921], [0.022, 0.923], [0.033, 0.912], [0.029, 0.905],
    [0.017, 0.900], [0.012, 0.895], [0.027, 0.893], [0.019, 0.886],
    [0.001, 0.883], [-0.012, 0.884], [-0.029, 0.883], [-0.038, 0.879],
    [-0.057, 0.881], [-0.062, 0.876], [-0.078, 0.876], [-0.087, 0.872],
    [-0.030, 0.907], [-0.007, 0.905], [-0.057, 0.916], [-0.025, 0.933],
    [-0.077, 0.990], [-0.059, 0.993]])

#: The user-specified triangles over ``BRITAIN``.
BRITAIN_TRIANGLES = np.asarray([
    [67, 66, 1], [65, 2, 66], [1, 66, 2], [64, 2, 65], [63, 3, 64],
    [60, 59, 57], [2, 64, 3], [3, 63, 4], [0, 67, 1], [62, 4, 63],
    [57, 59, 56], [59, 58, 56], [61, 60, 69], [57, 69, 60], [4, 62, 68],
    [6, 5, 9], [61, 68, 62], [69, 68, 61], [9, 5, 70], [6, 8, 7],
    [4, 70, 5], [8, 6, 9], [56, 69, 57], [69, 56, 52], [70, 10, 9],
    [54, 53, 55], [56, 55, 53], [68, 70, 4], [52, 56, 53], [11, 10, 12],
    [69, 71, 68], [68, 13, 70], [10, 70, 13], [51, 50, 52], [13, 68, 71],
    [52, 71, 69], [12, 10, 13], [71, 52, 50], [71, 14, 13], [50, 49, 71],
    [49, 48, 71], [14, 16, 15], [14, 71, 48], [17, 19, 18], [17, 20, 19],
    [48, 16, 14], [48, 47, 16], [47, 46, 16], [16, 46, 45], [23, 22, 24],
    [21, 24, 22], [17, 16, 45], [20, 17, 45], [21, 25, 24], [27, 26, 28],
    [20, 72, 21], [25, 21, 72], [45, 72, 20], [25, 28, 26], [44, 73, 45],
    [72, 45, 73], [28, 25, 29], [29, 25, 31], [43, 73, 44], [73, 43, 40],
    [72, 73, 39], [72, 31, 25], [42, 40, 43], [31, 30, 29], [39, 73, 40],
    [42, 41, 40], [72, 33, 31], [32, 31, 33], [39, 38, 72], [33, 72, 38],
    [33, 38, 34], [37, 35, 38], [34, 38, 35], [35, 37, 36]])


def _britain():
    """Longitudes and latitudes (degrees) of ``BRITAIN``."""
    return np.degrees(BRITAIN[:, 0]), np.degrees(BRITAIN[:, 1])


def _wales_bump(x, y):
    """The tri examples' bump over Wales, through libm's ``exp``: the last
    bits of its values decide the colour of a colorbar row."""
    return _libm_exp(-0.01 * ((x - -5) * (x - -5) + (y - 52) * (y - 52)))


def _britain_labels(ax, title):
    ax.set_title(title)
    ax.set_xlabel('Longitude (degrees)')
    ax.set_ylabel('Latitude (degrees)')


# --- synthetic stand-ins for the sample data -------------------------------

def _photo():
    """A 600 x 512 RGB uint8 picture in place of ``grace_hopper.jpg``."""
    yy, xx = np.mgrid[0:600, 0:512]
    r = (xx // 2) % 256
    g = yy * 255 // 599
    b = ((xx + 2 * yy) // 5) % 256
    return np.dstack([r, g, b]).astype(np.uint8)


def _scan():
    """A 256 x 256 MRI-like slice in place of ``s1045.ima.gz``: a bright
    ellipse with darker structures inside."""
    v = np.linspace(-1.0, 1.0, 256)
    X, Y = np.meshgrid(v, v)
    head = np.exp(-((X / 0.75)**2 + (Y / 0.9)**2)**4)
    folds = 0.5 + 0.5 * np.cos(14 * np.hypot(X - 0.1, Y + 0.05))
    return 1000.0 * head * (0.6 + 0.4 * folds)


def _logo():
    """A 130 x 542 RGBA float image in place of ``logo2.png``: coloured
    stripes on a transparent background."""
    yy, xx = np.mgrid[0:130, 0:542]
    rgba = np.zeros((130, 542, 4))
    rgba[..., 0] = (xx // 60) % 2
    rgba[..., 1] = ((xx // 60) % 4) / 4
    rgba[..., 2] = 1 - (yy // 26) / 4
    rgba[..., 3] = (((xx // 30) + (yy // 26)) % 3 != 0) * 1.0
    return rgba


def _terrain():
    """A rough 344 x 403 elevation model in place of
    ``jacksboro_fault_dem.npz``: ridges with noise on top."""
    yy, xx = np.mgrid[0:344, 0:403]
    ridges = 600 + 250 * np.sin(xx / 37.0) * np.cos(yy / 53.0) + 0.5 * (xx - yy)
    noise = rng(SEED).normal(344 * 403, scale=12.0).reshape(344, 403)
    return ridges + noise


# --- barbs -----------------------------------------------------------------

def _barb_points():
    return np.array([(-1.5, .5, -6, -6),
                     (1, -1, -46, 46),
                     (-3, -1, 11, -11),
                     (1, 1.5, 80, 80),
                     (0.5, 0.25, 25, 15),
                     (-1.5, -0.5, -5, 40)],
                    dtype=[('x', np.float32), ('y', np.float32),
                           ('u', np.float32), ('v', np.float32)])


@case("gallery_images/barb_demo_1")
def _(fig):
    x = np.linspace(-5, 5, 5)
    X, Y = np.meshgrid(x, x)
    U, V = 12 * X, 12 * Y
    data = _barb_points()
    axs1 = fig.subplots(nrows=2, ncols=2)
    axs1[0, 0].barbs(X, Y, U, V)
    axs1[0, 1].barbs(data['x'], data['y'], data['u'], data['v'], length=8, pivot='middle')
    axs1[1, 0].barbs(X, Y, U, V, np.sqrt(U ** 2 + V ** 2), fill_empty=True, rounding=False,
                     sizes=dict(emptybarb=0.25, spacing=0.2, height=0.3))
    axs1[1, 1].barbs(data['x'], data['y'], data['u'], data['v'], flagcolor='r',
                     barbcolor=['b', 'g'], flip_barb=True,
                     barb_increments=dict(half=10, full=20, flag=100))


@case("gallery_images/barb_demo_2")
def _(fig):
    data = _barb_points()
    masked_u = np.ma.masked_array(data['u'])
    masked_u[4] = 1000
    masked_u[4] = np.ma.masked
    ax2 = fig.subplots()
    ax2.barbs(data['x'], data['y'], masked_u, data['v'], length=8, pivot='middle')


# --- images ----------------------------------------------------------------

BARCODE = np.array([
    1, 0, 1, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1, 1,
    0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0,
    1, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 0, 1,
    1, 0, 0, 1, 1, 0, 1, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1])


@case("gallery_images/barcode_demo", figsize=(len(BARCODE) * 4 / 100, 2))
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axis_off()
    ax.imshow(BARCODE.reshape(1, -1), cmap='binary', aspect='auto', interpolation='nearest')


@case("gallery_images/colormap_interactive_adjustment")
def _(fig):
    t = np.linspace(0, 2 * np.pi, 1024)
    data2d = np.sin(t)[:, np.newaxis] * np.cos(t)[np.newaxis, :]
    ax = fig.subplots()
    im = ax.imshow(data2d)
    ax.set_title('Pan on the colorbar to shift the color mapping\n'
                 'Zoom on the colorbar to scale the color mapping')
    fig.colorbar(im, ax=ax, label='Interactive colorbar')


@case("gallery_images/demo_bboximage")
def _(fig):
    ax1, ax2 = fig.subplots(ncols=2)
    txt = ax1.text(0.5, 0.5, "test", size=30, ha="center", color="w")
    ax1.add_artist(BboxImage(txt.get_window_extent, data=np.arange(256).reshape((1, -1))))
    cmap_names = sorted(m for m in plt.colormaps if not m.endswith("_r"))
    ncol = 2
    nrow = len(cmap_names) // ncol + 1
    xpad_fraction = 0.3
    dx = 1 / (ncol + xpad_fraction * (ncol - 1))
    ypad_fraction = 0.3
    dy = 1 / (nrow + ypad_fraction * (nrow - 1))
    for i, cmap_name in enumerate(cmap_names):
        ix, iy = divmod(i, nrow)
        bbox0 = Bbox.from_bounds(ix * dx * (1 + xpad_fraction),
                                 1 - iy * dy * (1 + ypad_fraction) - dy, dx, dy)
        bbox = TransformedBbox(bbox0, ax2.transAxes)
        ax2.add_artist(BboxImage(bbox, cmap=cmap_name, data=np.arange(256).reshape((1, -1))))


@case("gallery_images/figimage_demo")
def _(fig):
    Z = np.arange(10000).reshape((100, 100))
    Z[:, 50:] = 1
    fig.figimage(Z, xo=50, yo=0, origin='lower')
    fig.figimage(Z, xo=100, yo=100, alpha=.8, origin='lower')


@case("gallery_images/image_clip_path")
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_photo())
    patch = Circle((260, 200), radius=200, transform=ax.transData)
    im.set_clip_path(patch)
    ax.axis('off')


def _demo_field():
    """``image_demo``'s grid: ``_two_bumps`` over [-3, 3)^2."""
    x = y = np.arange(-3.0, 3.0, 0.025)
    X, Y = np.meshgrid(x, y)
    return _two_bumps(X, Y)


@case("gallery_images/image_demo_1")
def _(fig):
    Z = _demo_field()
    ax = fig.subplots()
    ax.imshow(Z, interpolation='bilinear', cmap="RdYlBu", origin='lower', extent=[-3, 3, -3, 3],
              vmax=abs(Z).max(), vmin=-abs(Z).max())


@case("gallery_images/image_demo_2", figsize=(7, 3.5))
def _(fig):
    ax = fig.subplot_mosaic([['hopper', 'mri']])
    ax['hopper'].imshow(_photo())
    ax['hopper'].axis('off')
    ax['mri'].imshow(_scan(), cmap="hot", origin='upper', extent=(0, 25, 0, 25))
    markers = [(15.9, 14.5), (16.8, 15)]
    x, y = zip(*markers)
    ax['mri'].plot(x, y, 'o')
    ax['mri'].set_title('MRI')


@case("gallery_images/image_demo_3", figsize=(10, 3))
def _(fig):
    A = rng(SEED).uniform(25).reshape(5, 5)
    axs = fig.subplots(1, 3)
    for ax, interp in zip(axs, ['nearest', 'bilinear', 'bicubic']):
        ax.imshow(A, interpolation=interp)
        ax.set_title(interp.capitalize())
        ax.grid(True)


@case("gallery_images/image_demo_4", figsize=(3, 5))
def _(fig):
    x = np.arange(120).reshape((10, 12))
    axs = fig.subplots(nrows=2, sharex=True)
    axs[0].set_title('blue should be up')
    axs[0].imshow(x, origin='upper', interpolation='bilinear')
    axs[1].set_title('blue should be down')
    axs[1].imshow(x, origin='lower', interpolation='bilinear')


@case("gallery_images/image_demo_5")
def _(fig):
    Z = _demo_field()
    path = Path([[0, 1], [1, 0], [0, -1], [-1, 0], [0, 1]])
    patch = PathPatch(path, facecolor='none')
    ax = fig.subplots()
    ax.add_patch(patch)
    im = ax.imshow(Z, interpolation='bilinear', cmap="gray", origin='lower',
                   extent=[-3, 3, -3, 3], clip_path=patch, clip_on=True)
    im.set_clip_path(patch)


def _chirp(N, offset):
    """``image_antialiasing``'s chirp: rings whose frequency rises with the
    radius from the point ``(offset, offset)``."""
    x = np.arange(N) / N - offset
    X, Y = np.meshgrid(x, x)
    R = np.sqrt(X**2 + Y**2)
    return R, np.sin(np.pi * 2 * (5 * R + 100 * R**2 / 2))


def _exact_placement_images():
    """``image_exact_placement``'s A (100 x 300) and B (40 x 200)."""
    a = _chirp(450, 0.0)[1]
    A = a[:100, :300]
    return A, A[:40, :200]


def _annotate_rect(ax):
    ax.add_patch(Rectangle((0, 0), 200, 40, linewidth=1, edgecolor='r', facecolor='none'))


@case("gallery_images/image_exact_placement_1")
def _(fig):
    A, B = _exact_placement_images()
    fig.set_facecolor('aliceblue')
    axs = fig.subplots(1, 2)
    axs[0].imshow(A, vmin=-1, vmax=1)
    axs[1].imshow(B, vmin=-1, vmax=1)
    _annotate_rect(axs[0])


def _exact_placement_ratios(fig, anchor):
    A, B = _exact_placement_images()
    fig.set_facecolor('aliceblue')
    axs = fig.subplots(1, 2, width_ratios=[300 / 200, 1])
    axs[0].imshow(A, vmin=-1, vmax=1)
    _annotate_rect(axs[0])
    if anchor:
        axs[0].set_anchor(anchor)
    axs[1].imshow(B, vmin=-1, vmax=1)
    if anchor:
        axs[1].set_anchor(anchor)


@case("gallery_images/image_exact_placement_2", figsize=(6.4, 2))
def _(fig):
    _exact_placement_ratios(fig, None)


@case("gallery_images/image_exact_placement_3", figsize=(6.4, 2))
def _(fig):
    _exact_placement_ratios(fig, 'NW')


@case("gallery_images/image_exact_placement_4", figsize=(6.05, 1.7))
def _(fig):
    A, B = _exact_placement_images()
    fig.set_facecolor('aliceblue')
    fig_width, fig_height = 605, 170
    ax = fig.add_axes((35 / fig_width, 35 / fig_height, 300 / fig_width, 100 / fig_height))
    ax.imshow(A, vmin=-1, vmax=1)
    _annotate_rect(ax)
    ax = fig.add_axes((370 / fig_width, 95 / fig_height, 200 / fig_width, 40 / fig_height))
    ax.imshow(B, vmin=-1, vmax=1)


@case("gallery_images/image_masked", figsize=(6, 5.4))
def _(fig):
    x0, x1 = -5, 5
    y0, y1 = -3, 3
    X, Y = np.meshgrid(np.linspace(x0, x1, 500), np.linspace(y0, y1, 500))
    Z = _two_bumps(X, Y)
    palette = plt.colormaps["gray"].with_extremes(over='r', under='g', bad='b')
    Zm = np.ma.masked_where(Z > 1.2, Z)
    ax1, ax2 = fig.subplots(nrows=2)
    im = ax1.imshow(Zm, interpolation='bilinear', cmap=palette,
                    norm=colors.Normalize(vmin=-1.0, vmax=1.0), aspect='auto', origin='lower',
                    extent=[x0, x1, y0, y1])
    ax1.set_title('Green=low, Red=high, Blue=masked')
    cbar = fig.colorbar(im, extend='both', shrink=0.9, ax=ax1)
    cbar.set_label('uniform')
    ax1.tick_params(axis='x', labelbottom=False)
    im = ax2.imshow(Zm, interpolation='nearest', cmap=palette,
                    norm=colors.BoundaryNorm([-1, -0.5, -0.2, 0, 0.2, 0.5, 1],
                                             ncolors=palette.N),
                    aspect='auto', origin='lower', extent=[x0, x1, y0, y1])
    ax2.set_title('With BoundaryNorm')
    cbar = fig.colorbar(im, extend='both', spacing='proportional', shrink=0.9, ax=ax2)
    cbar.set_label('proportional')
    fig.suptitle('imshow, with out-of-range and masked data')


@case("gallery_images/image_nonuniform")
def _(fig):
    x = np.linspace(-4, 4, 9)
    x2 = x**3
    y = np.linspace(-4, 4, 9)
    z = np.sqrt(x[np.newaxis, :]**2 + y[:, np.newaxis]**2)
    fig.set_layout_engine('constrained')
    axs = fig.subplots(nrows=2, ncols=2)
    fig.suptitle('NonUniformImage class', fontsize='large')
    for row, interp in enumerate(['nearest', 'bilinear']):
        for ax, xs, half_width in [(axs[row, 0], x, 4), (axs[row, 1], x2, 64)]:
            im = NonUniformImage(ax, interpolation=interp,
                                 extent=(-half_width, half_width, -4, 4), cmap="Purples")
            im.set_data(xs, y, z)
            ax.add_image(im)
            ax.set_xlim(-half_width, half_width)
            ax.set_ylim(-4, 4)
            ax.set_title(interp)


def _blend_weights():
    """``image_transparency_blend``'s difference of two 2D Gaussian blobs."""
    def normal_pdf(x, mean, var):
        return np.exp(-(x - mean)**2 / (2 * var))

    xx = np.linspace(0, 100, 100)
    yy = np.linspace(0, 100, 100)
    gauss_x_high = normal_pdf(xx, 20, 150)
    gauss_y_high = normal_pdf(yy, 50, 150)
    gauss_x_low = normal_pdf(xx, 50, 200)
    gauss_y_low = normal_pdf(yy, 60, 200)
    return np.outer(gauss_y_high, gauss_x_high) - np.outer(gauss_y_low, gauss_x_low)


def _blend(fig, alphas):
    """The grey background, then the weights with ``alphas``; returns the
    axes and the weights."""
    weights = _blend_weights()
    greys = np.full((*weights.shape, 3), 70, dtype=np.uint8)
    vmax = np.abs(weights).max()
    ax = fig.subplots()
    ax.imshow(greys)
    ax.imshow(weights, alpha=alphas(weights), vmax=vmax, vmin=-vmax, cmap='RdYlBu',
              extent=(0, 100, 0, 100))
    return ax, weights


@case("gallery_images/image_transparency_blend_1")
def _(fig):
    ax, _ = _blend(fig, lambda weights: None)
    ax.set_axis_off()


def _fading(weights):
    alphas = np.ones(weights.shape)
    alphas[:, 30:] = np.linspace(1, 0, 70)
    return alphas


@case("gallery_images/image_transparency_blend_2")
def _(fig):
    ax, _ = _blend(fig, _fading)
    ax.set_axis_off()


def _by_magnitude(weights):
    return np.clip(colors.Normalize(0, .3, clip=True)(np.abs(weights)), .4, 1)


@case("gallery_images/image_transparency_blend_3")
def _(fig):
    ax, weights = _blend(fig, _by_magnitude)
    ax.contour(weights[::-1], levels=[-.1, .1], colors='k', linestyles='-')
    ax.set_axis_off()


@case("gallery_images/image_zcoord")
def _(fig):
    X = 10 * rng(SEED).uniform(15).reshape(5, 3)
    ax = fig.subplots()
    ax.imshow(X)


INTERPOLATION_METHODS = [
    None, 'none', 'nearest', 'bilinear', 'bicubic', 'spline16', 'spline36', 'hanning',
    'hamming', 'hermite', 'kaiser', 'quadric', 'catrom', 'gaussian', 'bessel', 'mitchell',
    'sinc', 'lanczos']


@case("gallery_images/interpolation_methods", figsize=(9, 6))
def _(fig):
    grid = rng(SEED).uniform(16).reshape(4, 4)
    axs = fig.subplots(nrows=3, ncols=6, subplot_kw={'xticks': [], 'yticks': []})
    for ax, interp_method in zip(axs.flat, INTERPOLATION_METHODS):
        ax.imshow(grid, interpolation=interp_method, cmap='viridis')
        ax.set_title(str(interp_method))
    fig.tight_layout()


@case("gallery_images/layer_images")
def _(fig):
    dx, dy = 0.05, 0.05
    x = np.arange(-3.0, 3.0, dx)
    y = np.arange(-3.0, 3.0, dy)
    X, Y = np.meshgrid(x, y)
    extent = np.min(x), np.max(x), np.min(y), np.max(y)
    fig.set_frameon(False)
    ax = fig.add_subplot()
    Z1 = np.add.outer(range(8), range(8)) % 2
    ax.imshow(Z1, cmap="gray", interpolation='nearest', extent=extent)
    ax.imshow(_func3(X, Y), cmap="viridis", alpha=.9, interpolation='bilinear', extent=extent)


@case("gallery_images/matshow", figsize=(4.8, 4.8))
def _(fig):
    ax = fig.add_axes((0.15, 0.09, 0.775, 0.775))
    ax.matshow(np.diag(range(15)))


@case("gallery_images/multi_image")
def _(fig):
    g = rng(SEED)
    datasets = [(i + 1) / 10 * g.uniform(200).reshape(10, 20) for i in range(4)]
    axs = fig.subplots(2, 2)
    fig.suptitle('Multiple images')
    norm = colors.Normalize(vmin=np.min(datasets), vmax=np.max(datasets))
    colorizer = mpl.colorizer.Colorizer(norm=norm)
    images = [ax.imshow(data, colorizer=colorizer) for ax, data in zip(axs.flat, datasets)]
    fig.colorbar(images[0], ax=axs, orientation='horizontal', fraction=.1)


@case("gallery_images/shading_example_1")
def _(fig):
    x, y = np.mgrid[-5:5:0.05, -5:5:0.05]
    z = 5 * (np.sqrt(x**2 + y**2) + np.sin(x**2 + y**2))
    _compare_shading(fig, z, plt.colormaps["copper"])
    fig.suptitle('HSV Blending Looks Best with Smooth Surfaces', y=0.95)


@case("gallery_images/shading_example_2")
def _(fig):
    _compare_shading(fig, _terrain(), plt.colormaps["gist_earth"], ve=0.05)
    fig.suptitle('Overlay Blending Looks Best with Rough Surfaces', y=0.95)


def _compare_shading(fig, z, cmap, ve=1):
    axs = fig.subplots(ncols=2, nrows=2)
    for ax in axs.flat:
        ax.set(xticks=[], yticks=[])
    ls = colors.LightSource(azdeg=315, altdeg=45)
    axs[0, 0].imshow(z, cmap=cmap)
    axs[0, 0].set(xlabel='Colormapped Data')
    axs[0, 1].imshow(ls.hillshade(z, vert_exag=ve), cmap='gray')
    axs[0, 1].set(xlabel='Illumination Intensity')
    axs[1, 0].imshow(ls.shade(z, cmap=cmap, vert_exag=ve, blend_mode='hsv'))
    axs[1, 0].set(xlabel='Blend Mode: "hsv" (default)')
    axs[1, 1].imshow(ls.shade(z, cmap=cmap, vert_exag=ve, blend_mode='overlay'))
    axs[1, 1].set(xlabel='Blend Mode: "overlay"')


@case("gallery_images/specgram_demo")
def _(fig):
    dt = 0.0005
    t = np.arange(0.0, 20.5, dt)
    s1 = np.sin(2 * np.pi * 100 * t)
    s2 = 2 * np.sin(2 * np.pi * 400 * t)
    s2[t <= 10] = s2[12 <= t] = 0
    nse = 0.01 * rng(SEED).uniform(len(t))
    x = s1 + s2 + nse
    ax1, ax2 = fig.subplots(nrows=2, sharex=True)
    ax1.plot(t, x)
    ax1.set_ylabel('Signal')
    ax2.specgram(x, NFFT=1024, Fs=1 / dt)
    ax2.set_xlabel('Time (s)')
    ax2.set_ylabel('Frequency (Hz)')
    ax2.set_xlim(0, 20)


@case("gallery_images/spy_demos")
def _(fig):
    axs = fig.subplots(2, 2)
    x = rng(SEED).normal(400).reshape(20, 20)
    x[5, :] = 0.
    x[:, 12] = 0.
    axs[0, 0].spy(x, markersize=5)
    axs[0, 1].spy(x, precision=0.1, markersize=5)
    axs[1, 0].spy(x)
    axs[1, 1].spy(x, precision=0.1)


@case("gallery_images/watermark_image")
def _(fig):
    ax = fig.subplots()
    x = np.arange(30)
    y = x + rng(SEED).normal(30)
    ax.bar(x, y, color='#6bbc6b')
    ax.grid()
    fig.figimage(_logo(), 25, 25, zorder=3, alpha=.7)


# --- image_antialiasing ----------------------------------------------------

def _alarge():
    """``image_antialiasing``'s 450 x 450 test pattern: alternating rows on
    the left third, a chirp with two flat discs elsewhere."""
    N = 450
    aa = np.ones((N, N))
    aa[::2, :] = -1
    R, a = _chirp(N, 0.5)
    a[:int(N / 2), :][R[:int(N / 2), :] < 0.4] = -1
    a[:int(N / 2), :][R[:int(N / 2), :] < 0.3] = 1
    aa[:, int(N / 3):] = a[:, int(N / 3):]
    return aa


def _asmall():
    return rng(SEED + 9).uniform(16).reshape(4, 4)


@case("gallery_images/image_antialiasing_1", figsize=(4, 2))
def _(fig):
    axs = fig.subplots(1, 2)
    axs[0].imshow(_alarge(), cmap='RdBu_r')
    axs[0].set_title('(450, 450) Down-sampled', fontsize='medium')
    axs[1].imshow(_asmall(), cmap='viridis')
    axs[1].set_title('(4, 4) Up-sampled', fontsize='medium')


@case("gallery_images/image_antialiasing_2", figsize=(4, 4))
def _(fig):
    fig.set_layout_engine('compressed')
    ax = fig.subplots()
    ax.imshow(_alarge(), interpolation='nearest', cmap='RdBu_r')
    ax.set_xlim(100, 200)
    ax.set_ylim(275, 175)
    ax.set_title('Zoom')


def _stages(fig, interp):
    fig.set_layout_engine('compressed')
    axs = fig.subplots(1, 2)
    for ax, space in zip(axs.flat, ['data', 'rgba']):
        ax.imshow(_alarge(), interpolation=interp, interpolation_stage=space, cmap='RdBu_r')
        ax.set_title(f"interpolation='{interp}'\nstage='{space}'")


@case("gallery_images/image_antialiasing_3", figsize=(5, 2.7))
def _(fig):
    _stages(fig, 'nearest')


@case("gallery_images/image_antialiasing_4", figsize=(5, 2.7))
def _(fig):
    _stages(fig, 'hanning')


def _upsampled(fig, interp):
    ax = fig.subplots()
    ax.imshow(_alarge(), interpolation=interp, cmap='grey')
    ax.set_title(f"up-sampled by factor a 1.17, interpolation='{interp}'")


@case("gallery_images/image_antialiasing_5", figsize=(6.8, 6.8))
def _(fig):
    _upsampled(fig, 'nearest')


@case("gallery_images/image_antialiasing_6", figsize=(6.8, 6.8))
def _(fig):
    _upsampled(fig, 'auto')


@case("gallery_images/image_antialiasing_7", figsize=(7, 4))
def _(fig):
    fig.set_layout_engine('constrained')
    axs = fig.subplots(1, 2)
    for ax, interp in zip(axs, ['hanning', 'lanczos']):
        ax.imshow(_alarge(), interpolation=interp, cmap='gray')
        ax.set_title(f"interpolation='{interp}'")


@case("gallery_images/image_antialiasing_8", figsize=(7, 3))
def _(fig):
    a = _alarge() + 1
    cmap = plt.get_cmap('RdBu_r')
    cmap.set_under('yellow')
    cmap.set_over('limegreen')
    fig.set_layout_engine('constrained')
    axs = fig.subplots(1, 3)
    for ax, interp, space in zip(axs.flat, ['hanning', 'nearest', 'hanning'],
                                 ['data', 'data', 'rgba']):
        im = ax.imshow(a, interpolation=interp, interpolation_stage=space, cmap=cmap,
                       vmin=0, vmax=2)
        title = f"interpolation='{interp}'\nstage='{space}'"
        if ax == axs[2]:
            title += '\nDefault'
        ax.set_title(title, fontsize='medium')
    fig.colorbar(im, ax=axs, extend='both', shrink=0.8)


@case("gallery_images/image_antialiasing_9", figsize=(6.5, 4))
def _(fig):
    asmall = _asmall()
    fig.set_layout_engine('compressed')
    axs = fig.subplots(1, 2)
    axs[0].imshow(asmall, cmap='viridis')
    axs[0].set_title("interpolation='auto'\nstage='auto'")
    axs[1].imshow(asmall, cmap='viridis', interpolation="nearest", interpolation_stage="data")
    axs[1].set_title("interpolation='nearest'\nstage='data'")


@case("gallery_images/image_antialiasing_10", figsize=(6.5, 4))
def _(fig):
    a = _asmall()
    fig.set_layout_engine('compressed')
    axs = fig.subplots(1, 2)
    im = axs[0].imshow(a, cmap='viridis', interpolation='sinc', interpolation_stage='data')
    axs[0].set_title("interpolation='sinc'\nstage='data'\n(default for upsampling)")
    axs[1].imshow(a, cmap='viridis', interpolation='sinc', interpolation_stage='rgba')
    axs[1].set_title("interpolation='sinc'\nstage='rgba'")
    fig.colorbar(im, ax=axs, shrink=0.7, extend='both')


@case("gallery_images/image_antialiasing_11", figsize=(2, 2))
def _(fig):
    ax = fig.add_axes((0, 0, 1, 1))
    ax.imshow(_alarge()[:400, :400], cmap='RdBu_r', interpolation='nearest')


# --- heatmaps --------------------------------------------------------------

VEGETABLES = ["cucumber", "tomato", "lettuce", "asparagus", "potato", "wheat", "barley"]
FARMERS = ["Farmer Joe", "Upland Bros.", "Smith Gardening", "Agrifun", "Organiculture",
           "BioGoods Ltd.", "Cornylee Corp."]
HARVEST = np.array([[0.8, 2.4, 2.5, 3.9, 0.0, 4.0, 0.0],
                    [2.4, 0.0, 4.0, 1.0, 2.7, 0.0, 0.0],
                    [1.1, 2.4, 0.8, 4.3, 1.9, 4.4, 0.0],
                    [0.6, 0.0, 0.3, 0.0, 3.1, 0.0, 0.0],
                    [0.7, 1.7, 0.6, 2.6, 2.2, 6.2, 0.0],
                    [1.3, 1.2, 0.0, 0.0, 0.0, 3.2, 5.1],
                    [0.1, 2.0, 0.0, 1.4, 0.0, 1.9, 6.3]])


@case("gallery_images/image_annotated_heatmap_1")
def _(fig):
    ax = fig.subplots()
    ax.imshow(HARVEST)
    ax.set_xticks(range(len(FARMERS)), labels=FARMERS, rotation=45, rotation_mode="xtick")
    ax.set_yticks(range(len(VEGETABLES)), labels=VEGETABLES)
    for i in range(len(VEGETABLES)):
        for j in range(len(FARMERS)):
            ax.text(j, i, HARVEST[i, j], ha="center", va="center", color="w")
    ax.set_title("Harvest of local farmers (in tons/year)")
    fig.tight_layout()


def _heatmap(data, row_labels, col_labels, ax, cbar_kw=None, cbarlabel="", **kwargs):
    im = ax.imshow(data, **kwargs)
    cbar = ax.figure.colorbar(im, ax=ax, **(cbar_kw or {}))
    cbar.ax.set_ylabel(cbarlabel, rotation=-90, va="bottom")
    ax.set_xticks(range(data.shape[1]), labels=col_labels, rotation=-30, rotation_mode="xtick")
    ax.set_yticks(range(data.shape[0]), labels=row_labels)
    ax.tick_params(top=True, bottom=False, labeltop=True, labelbottom=False)
    ax.spines[:].set_visible(False)
    ax.set_xticks(np.arange(data.shape[1] + 1) - .5, minor=True)
    ax.set_yticks(np.arange(data.shape[0] + 1) - .5, minor=True)
    ax.grid(which="minor", color="w", linestyle='-', linewidth=3)
    ax.tick_params(which="minor", bottom=False, left=False)
    return im


def _annotate_heatmap(im, valfmt="{x:.2f}", textcolors=("black", "white"), threshold=None,
                      **textkw):
    data = im.get_array()
    if threshold is not None:
        threshold = im.norm(threshold)
    else:
        threshold = im.norm(data.max()) / 2.
    kw = dict(horizontalalignment="center", verticalalignment="center")
    kw.update(textkw)
    if isinstance(valfmt, str):
        valfmt = ticker.StrMethodFormatter(valfmt)
    for i in range(data.shape[0]):
        for j in range(data.shape[1]):
            kw.update(color=textcolors[int(im.norm(data[i, j]) > threshold)])
            im.axes.text(j, i, valfmt(data[i, j], None), **kw)


@case("gallery_images/image_annotated_heatmap_2")
def _(fig):
    ax = fig.subplots()
    im = _heatmap(HARVEST, VEGETABLES, FARMERS, ax=ax, cmap="YlGn", cbarlabel="harvest [t/year]")
    _annotate_heatmap(im, valfmt="{x:.1f} t")
    fig.tight_layout()


def _correlation(x, pos):
    return f"{x:.2f}".replace("0.", ".").replace("1.00", "")


@case("gallery_images/image_annotated_heatmap_3", figsize=(8, 6))
def _(fig):
    g = rng(SEED)
    (ax, ax2), (ax3, ax4) = fig.subplots(2, 2)
    im = _heatmap(HARVEST, VEGETABLES, FARMERS, ax=ax, cmap="Wistia",
                  cbarlabel="harvest [t/year]")
    _annotate_heatmap(im, valfmt="{x:.1f}", size=7)

    data = g.integers(49, 2, 100).reshape(7, 7)
    y = [f"Book {i}" for i in range(1, 8)]
    x = [f"Store {i}" for i in list("ABCDEFG")]
    im = _heatmap(data, y, x, ax=ax2, vmin=0, cmap="magma_r", cbarlabel="weekly sold copies")
    _annotate_heatmap(im, valfmt="{x:d}", size=7, threshold=20, textcolors=("red", "white"))

    data = g.normal(36).reshape(6, 6)
    y = [f"Prod. {i}" for i in range(10, 70, 10)]
    x = [f"Cycle {i}" for i in range(1, 7)]
    qrates = list("ABCDEFG")
    norm = colors.BoundaryNorm(np.linspace(-3.5, 3.5, 8), 7)
    fmt = ticker.FuncFormatter(lambda x, pos: qrates[::-1][norm(x)])
    im = _heatmap(data, y, x, ax=ax3, cmap=mpl.colormaps["PiYG"].resampled(7), norm=norm,
                  cbar_kw=dict(ticks=np.arange(-3, 4), format=fmt), cbarlabel="Quality Rating")
    _annotate_heatmap(im, valfmt=fmt, size=9, fontweight="bold", threshold=-1,
                      textcolors=("red", "black"))

    im = _heatmap(np.corrcoef(HARVEST), VEGETABLES, VEGETABLES, ax=ax4, cmap="PuOr", vmin=-1,
                  vmax=1, cbarlabel="correlation coeff.")
    _annotate_heatmap(im, valfmt=ticker.FuncFormatter(_correlation), size=7)
    fig.tight_layout()


# --- colormap normalisations -----------------------------------------------

def _norm_pair(fig, plot, norm, extend, label):
    """``colormap_normalizations``' two rows: linear scaling, then ``norm``,
    each with a colorbar."""
    ax = fig.subplots(2, 1)
    pcm = plot(ax[0], {})
    fig.colorbar(pcm, ax=ax[0], extend=extend, label='linear scaling')
    pcm = plot(ax[1], {'norm': norm})
    fig.colorbar(pcm, ax=ax[1], extend=extend, label=label)


def _mgrid_100(x0, x1, y0, y1):
    return np.mgrid[x0:x1:complex(0, 100), y0:y1:complex(0, 100)]


def _bipolar():
    """``colormap_normalizations``' field of a large positive and a small
    negative bump."""
    X, Y = _mgrid_100(-3, 3, -2, 2)
    Z1 = np.exp(-X**2 - Y**2)
    Z2 = np.exp(-(X - 1)**2 - (Y - 1)**2)
    return X, Y, (5 * Z1 - Z2) * 2


@case("gallery_images/colormap_normalizations_1")
def _(fig):
    X, Y = _mgrid_100(-3, 3, -2, 2)
    Z = _spike(X, Y)
    _norm_pair(fig, lambda ax, kw: ax.pcolor(X, Y, Z, cmap='PuBu_r', shading='nearest', **kw),
               colors.LogNorm(vmin=Z.min(), vmax=Z.max()), 'max', 'LogNorm')


@case("gallery_images/colormap_normalizations_2")
def _(fig):
    X, Y = _mgrid_100(0, 3, 0, 2)
    Z = (1 + np.sin(Y * 10)) * X**2
    _norm_pair(fig,
               lambda ax, kw: ax.pcolormesh(X, Y, Z, cmap='PuBu_r', shading='nearest', **kw),
               colors.PowerNorm(gamma=0.5), 'max', 'PowerNorm')


def _bipolar_mesh(X, Y, Z):
    def plot(ax, kw):
        kw = kw or {'vmin': -np.max(Z)}
        return ax.pcolormesh(X, Y, Z, cmap='RdBu_r', shading='nearest', **kw)
    return plot


@case("gallery_images/colormap_normalizations_3")
def _(fig):
    X, Y, Z = _bipolar()
    _norm_pair(fig, _bipolar_mesh(X, Y, Z),
               colors.SymLogNorm(linthresh=0.015, vmin=-10.0, vmax=10.0, base=10), 'both',
               'SymLogNorm')


class MidpointNormalize(colors.Normalize):
    """``colormap_normalizations``' custom norm: linear on each side of
    ``midpoint``."""

    def __init__(self, vmin=None, vmax=None, midpoint=None, clip=False):
        self.midpoint = midpoint
        super().__init__(vmin, vmax, clip)

    def __call__(self, value, clip=None):
        x, y = [self.vmin, self.midpoint, self.vmax], [0, 0.5, 1]
        return np.ma.masked_array(np.interp(value, x, y))


@case("gallery_images/colormap_normalizations_4")
def _(fig):
    X, Y, Z = _bipolar()
    _norm_pair(fig, _bipolar_mesh(X, Y, Z), MidpointNormalize(midpoint=0), 'both',
               'Custom norm')


@case("gallery_images/colormap_normalizations_5")
def _(fig):
    X, Y, Z = _bipolar()
    fig.set_layout_engine('constrained')
    ax = fig.subplots(3, 1)
    pcm = ax[0].pcolormesh(X, Y, Z, cmap='RdBu_r', shading='nearest', vmin=-np.max(Z))
    fig.colorbar(pcm, ax=ax[0], extend='both', orientation='vertical', label='linear scaling')
    for axis, bounds, label in [
        (ax[1], np.linspace(-2, 2, 11), 'BoundaryNorm\nlinspace(-2, 2, 11)'),
        (ax[2], np.array([-1, -0.5, 0, 2.5, 5]), 'BoundaryNorm\n[-1, -0.5, 0, 2.5, 5]'),
    ]:
        norm = colors.BoundaryNorm(boundaries=bounds, ncolors=256)
        pcm = axis.pcolormesh(X, Y, Z, cmap='RdBu_r', shading='nearest', norm=norm)
        fig.colorbar(pcm, ax=axis, extend='both', orientation='vertical', label=label)


def _rbf_field():
    """``colormap_normalizations_symlognorm``'s field, on 40 x 40 points
    instead of 200 x 200: the example's 80,000 Gouraud triangles per mesh
    make SVGs of half a gigabyte."""
    def rbf(x, y):
        return 1.0 / (1 + 5 * ((x ** 2) + (y ** 2)))

    X, Y = np.mgrid[-3:3:complex(0, 40), -2:2:complex(0, 40)]
    return X, Y, 8 * rbf(X + 0.5, Y + 0.5) - rbf(X - 0.5, Y - 0.5)


def _symlog_against(fig, mapping, label):
    X, Y, Z = _rbf_field()
    ax = fig.subplots(2, 1, sharex=True, sharey=True)
    pcm = ax[0].pcolormesh(X, Y, Z, cmap='PRGn', shading='gouraud',
                           norm=colors.SymLogNorm(linthresh=0.5, linscale=1, vmin=-8, vmax=8,
                                                  base=10))
    fig.colorbar(pcm, ax=ax[0], extend='both')
    ax[0].text(-2.5, 1.5, 'symlog')
    pcm = ax[1].pcolormesh(X, Y, Z, cmap='PRGn', shading='gouraud', **mapping)
    fig.colorbar(pcm, ax=ax[1], extend='both')
    ax[1].text(-2.5, 1.5, label)


@case("gallery_images/colormap_normalizations_symlognorm_1")
def _(fig):
    _symlog_against(fig, {'vmin': -8, 'vmax': 8}, 'linear')


@case("gallery_images/colormap_normalizations_symlognorm_2")
def _(fig):
    _symlog_against(fig, {'norm': colors.AsinhNorm(linear_width=0.5, vmin=-8, vmax=8)}, 'asinh')


# --- pcolor and pcolormesh -------------------------------------------------

def _random_6x10():
    return rng(SEED).uniform(60).reshape(6, 10)


@case("gallery_images/pcolor_demo_1")
def _(fig):
    Z = _random_6x10()
    ax0, ax1 = fig.subplots(2, 1)
    ax0.pcolor(Z)
    ax0.set_title('default: no edges')
    ax1.pcolor(Z, edgecolors='k', linewidths=4)
    ax1.set_title('thick edges')
    fig.tight_layout()


@case("gallery_images/pcolor_demo_2")
def _(fig):
    dx, dy = 0.15, 0.05
    y, x = np.mgrid[-3:3 + dy:dy, -3:3 + dx:dx]
    z = _func3(x, y)[:-1, :-1]
    z_min, z_max = -abs(z).max(), abs(z).max()
    axs = fig.subplots(2, 2)
    common = dict(cmap='RdBu', vmin=z_min, vmax=z_max)
    for ax, title, mappable in [
        (axs[0, 0], 'pcolor', lambda ax: ax.pcolor(x, y, z, **common)),
        (axs[0, 1], 'pcolormesh', lambda ax: ax.pcolormesh(x, y, z, **common)),
        (axs[1, 0], 'image (nearest, aspect="auto")',
         lambda ax: ax.imshow(z, extent=[x.min(), x.max(), y.min(), y.max()],
                              interpolation='nearest', origin='lower', aspect='auto', **common)),
        (axs[1, 1], 'pcolorfast', lambda ax: ax.pcolorfast(x, y, z, **common)),
    ]:
        c = mappable(ax)
        ax.set_title(title)
        fig.colorbar(c, ax=ax)
    fig.tight_layout()


@case("gallery_images/pcolor_demo_3")
def _(fig):
    X, Y = np.meshgrid(np.linspace(-3, 3, 100), np.linspace(-2, 2, 100))
    Z = _spike(X, Y)
    ax0, ax1 = fig.subplots(2, 1)
    c = ax0.pcolor(X, Y, Z, shading='auto', norm=colors.LogNorm(vmin=Z.min(), vmax=Z.max()),
                   cmap='PuBu_r')
    fig.colorbar(c, ax=ax0)
    c = ax1.pcolor(X, Y, Z, cmap='PuBu_r', shading='auto')
    fig.colorbar(c, ax=ax1)


GRID_Z = np.arange(15).reshape(3, 5)


def _annotate_grid(ax, x, y, title):
    X, Y = np.meshgrid(x, y)
    ax.plot(X.flat, Y.flat, 'o', color='m')
    ax.set_xlim(-0.7, 5.2)
    ax.set_ylim(-0.7, 3.2)
    ax.set_title(title)


def _grid_mesh(ax, x, y, Z, shading, title):
    ax.pcolormesh(x, y, Z, shading=shading, vmin=GRID_Z.min(), vmax=GRID_Z.max())
    _annotate_grid(ax, x, y, title)


@case("gallery_images/pcolormesh_grids_1")
def _(fig):
    _grid_mesh(fig.subplots(), np.arange(6), np.arange(4), GRID_Z, 'flat', "shading='flat'")


@case("gallery_images/pcolormesh_grids_2")
def _(fig):
    _grid_mesh(fig.subplots(), np.arange(5), np.arange(3), GRID_Z[:-1, :-1], 'flat',
               "shading='flat': X, Y, C same shape")


@case("gallery_images/pcolormesh_grids_3")
def _(fig):
    _grid_mesh(fig.subplots(), np.arange(5), np.arange(3), GRID_Z, 'nearest',
               "shading='nearest'")


@case("gallery_images/pcolormesh_grids_4")
def _(fig):
    fig.set_layout_engine('constrained')
    axs = fig.subplots(2, 1)
    _grid_mesh(axs[0], np.arange(5), np.arange(3), GRID_Z, 'auto',
               "shading='auto'; X, Y, Z: same shape (nearest)")
    _grid_mesh(axs[1], np.arange(6), np.arange(4), GRID_Z, 'auto',
               "shading='auto'; X, Y one larger than Z (flat)")


@case("gallery_images/pcolormesh_grids_5")
def _(fig):
    fig.set_layout_engine('constrained')
    _grid_mesh(fig.subplots(), np.arange(5), np.arange(3), GRID_Z, 'gouraud',
               "shading='gouraud'; X, Y same shape as Z")


@case("gallery_images/pcolormesh_levels_1")
def _(fig):
    ax = fig.subplots()
    ax.pcolormesh(np.arange(-0.5, 10, 1), np.arange(4.5, 11, 1), _random_6x10())


@case("gallery_images/pcolormesh_levels_2")
def _(fig):
    X, Y = np.meshgrid(np.arange(-0.5, 10, 1), np.arange(4.5, 11, 1))
    X = X + 0.2 * Y
    Y = Y + 0.3 * X
    ax = fig.subplots()
    ax.pcolormesh(X, Y, _random_6x10())


@case("gallery_images/pcolormesh_levels_3")
def _(fig):
    Z = _random_6x10()
    X, Y = np.meshgrid(np.arange(10), np.arange(6))
    axs = fig.subplots(2, 1, sharex=True, sharey=True)
    axs[0].pcolormesh(X, Y, Z, vmin=np.min(Z), vmax=np.max(Z), shading='auto')
    axs[0].set_title("shading='auto' = 'nearest'")
    axs[1].pcolormesh(X, Y, Z[:-1, :-1], vmin=np.min(Z), vmax=np.max(Z), shading='flat')
    axs[1].set_title("shading='flat'")


@case("gallery_images/pcolormesh_levels_4")
def _(fig):
    dx, dy = 0.05, 0.05
    y, x = np.mgrid[slice(1, 5 + dy, dy), slice(1, 5 + dx, dx)]
    z = (np.sin(x)**10 + np.cos(10 + y * x) * np.cos(x))[:-1, :-1]
    levels = ticker.MaxNLocator(nbins=15).tick_values(z.min(), z.max())
    cmap = plt.colormaps['PiYG']
    norm = colors.BoundaryNorm(levels, ncolors=cmap.N, clip=True)
    ax0, ax1 = fig.subplots(nrows=2)
    im = ax0.pcolormesh(x, y, z, cmap=cmap, norm=norm)
    fig.colorbar(im, ax=ax0)
    ax0.set_title('pcolormesh with levels')
    cf = ax1.contourf(x[:-1, :-1] + dx / 2., y[:-1, :-1] + dy / 2., z, levels=levels, cmap=cmap)
    fig.colorbar(cf, ax=ax1)
    ax1.set_title('contourf with levels')
    fig.tight_layout()


@case("gallery_images/quadmesh_demo")
def _(fig):
    n = 12
    X, Y = np.meshgrid(np.linspace(-1.5, 1.5, n), np.linspace(-1.5, 1.5, n * 2))
    Qx = np.cos(Y) - np.cos(X)
    Qz = np.sin(Y) + np.sin(X)
    Z = np.sqrt(X**2 + Y**2) / 5
    Z = (Z - Z.min()) / (Z.max() - Z.min())
    Zm = np.ma.masked_where(np.abs(Qz) < 0.5 * np.max(Qz), Z)
    axs = fig.subplots(nrows=1, ncols=3)
    axs[0].pcolormesh(Qx, Qz, Z, shading='gouraud')
    axs[0].set_title('Without masked values')
    cmap = plt.colormaps[plt.rcParams['image.cmap']].with_extremes(bad='y')
    axs[1].pcolormesh(Qx, Qz, Zm, shading='gouraud', cmap=cmap)
    axs[1].set_title('With masked values')
    axs[2].pcolormesh(Qx, Qz, Zm, shading='gouraud')
    axs[2].set_title('With masked values')
    fig.tight_layout()


# --- contours --------------------------------------------------------------

@case("gallery_images/contour_corner_mask")
def _(fig):
    x, y = np.meshgrid(np.arange(7), np.arange(10))
    z = np.sin(0.5 * x) * np.cos(0.52 * y)
    mask = np.zeros_like(z, dtype=bool)
    mask[2, 3:5] = True
    mask[3:5, 4] = True
    mask[7, 2] = True
    mask[5, 0] = True
    mask[0, 6] = True
    z = np.ma.array(z, mask=mask)
    axs = fig.subplots(ncols=2)
    for ax, corner_mask in zip(axs, [False, True]):
        cs = ax.contourf(x, y, z, corner_mask=corner_mask)
        ax.contour(cs, colors='k')
        ax.set_title(f'{corner_mask=}')
        ax.grid(c='k', ls='-', alpha=0.3)
        ax.plot(np.ma.array(x, mask=~mask), y, 'ro')


@case("gallery_images/contour_demo_1")
def _(fig):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z)
    ax.clabel(CS, fontsize=10)
    ax.set_title('Simplest default with labels')


@case("gallery_images/contour_demo_2")
def _(fig):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z)
    manual_locations = [
        (-1, -1.4), (-0.62, -0.7), (-2, 0.5), (1.7, 1.2), (2.0, 1.4), (2.4, 1.7)]
    ax.clabel(CS, fontsize=10, manual=manual_locations)
    ax.set_title('labels at selected locations')


def _single_color(fig, title):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z, 6, colors='k')
    ax.clabel(CS, fontsize=9)
    ax.set_title(title)


@case("gallery_images/contour_demo_3")
def _(fig):
    _single_color(fig, 'Single color - negative contours dashed')


#: ``contour_demo`` sets this before its fourth figure and keeps it.
SOLID_NEGATIVE = {'contour.negative_linestyle': 'solid'}


@case("gallery_images/contour_demo_4", rc=SOLID_NEGATIVE)
def _(fig):
    _single_color(fig, 'Single color - negative contours solid')


@case("gallery_images/contour_demo_5", rc=SOLID_NEGATIVE)
def _(fig):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z, 6, linewidths=np.arange(.5, 4, .5),
                    colors=('r', 'green', 'blue', (1, 1, 0), '#afeeee', '0.5'))
    ax.clabel(CS, fontsize=9)
    ax.set_title('Crazy lines')


@case("gallery_images/contour_demo_6", rc=SOLID_NEGATIVE)
def _(fig):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    im = ax.imshow(Z, interpolation='bilinear', origin='lower', cmap="gray", extent=(-3, 3, -2, 2))
    levels = np.arange(-1.2, 1.6, 0.2)
    CS = ax.contour(Z, levels, origin='lower', cmap='flag', extend='both', linewidths=2,
                    extent=(-3, 3, -2, 2))
    lws = np.resize(CS.get_linewidth(), len(levels))
    lws[6] = 4
    CS.set_linewidth(lws)
    ax.clabel(CS, levels[1::2], fmt='%1.1f', fontsize=14)
    CB = fig.colorbar(CS, shrink=0.8)
    ax.set_title('Lines with colorbar')
    fig.colorbar(im, orientation='horizontal', shrink=0.8)
    l, b, w, h = ax.get_position().bounds
    ll, bb, ww, hh = CB.ax.get_position().bounds
    CB.ax.set_position([ll, b + 0.1 * h, ww, h * 0.8])


def _holey_field():
    """``contourf_demo``'s field with a NaN corner, a masked corner and a
    masked disc."""
    x = y = np.arange(-3.0, 3.01, 0.025)
    X, Y = np.meshgrid(x, y)
    Z = _two_bumps(X, Y)
    nr, nc = Z.shape
    Z[-nr // 6:, -nc // 6:] = np.nan
    Z = np.ma.array(Z)
    Z[:nr // 6, :nc // 6] = np.ma.masked
    Z[np.sqrt(X**2 + Y**2) < 0.5] = np.ma.masked
    return X, Y, Z


HOLEY_LEVELS = [-1.5, -1, -0.5, 0, 0.5, 1]


@case("gallery_images/contourf_demo_1")
def _(fig):
    X, Y, Z = _holey_field()
    fig.set_layout_engine('constrained')
    ax2 = fig.subplots()
    CS = ax2.contourf(X, Y, Z, 10, cmap="bone")
    CS2 = ax2.contour(CS, levels=CS.levels[::2], colors='r')
    ax2.set_title('Nonsense (3 masked regions)')
    ax2.set_xlabel('word length anomaly')
    ax2.set_ylabel('sentence length anomaly')
    cbar = fig.colorbar(CS)
    cbar.ax.set_ylabel('verbosity coefficient')
    cbar.add_lines(CS2)


@case("gallery_images/contourf_demo_2")
def _(fig):
    X, Y, Z = _holey_field()
    fig.set_layout_engine('constrained')
    ax2 = fig.subplots()
    CS3 = ax2.contourf(X, Y, Z, HOLEY_LEVELS, colors=('r', 'g', 'b'), extend='both')
    CS3.cmap.set_under('yellow')
    CS3.cmap.set_over('cyan')
    CS4 = ax2.contour(X, Y, Z, HOLEY_LEVELS, colors=('k',), linewidths=(3,))
    ax2.set_title('Listed colors (3 masked regions)')
    ax2.clabel(CS4, fmt='%2.1f', colors='w', fontsize=14)
    fig.colorbar(CS3)


@case("gallery_images/contourf_demo_3")
def _(fig):
    X, Y, Z = _holey_field()
    cmap = plt.colormaps["winter"].with_extremes(under="magenta", over="yellow")
    fig.set_layout_engine('constrained')
    axs = fig.subplots(2, 2)
    for ax, extend in zip(axs.flat, ["neither", "both", "min", "max"]):
        cs = ax.contourf(X, Y, Z, HOLEY_LEVELS, cmap=cmap, extend=extend)
        fig.colorbar(cs, ax=ax, shrink=0.9)
        ax.set_title("extend = %s" % extend)
        ax.locator_params(nbins=4)


@case("gallery_images/contourf_demo_4")
def _(fig):
    x = np.arange(1, 10)
    h = x * x.reshape(-1, 1)
    ax1, ax2 = fig.subplots(ncols=2)
    ax1.set_title("origin='upper'")
    ax2.set_title("origin='lower'")
    ax1.contourf(h, levels=np.arange(5, 70, 5), extend='both', origin="upper")
    ax2.contourf(h, levels=np.arange(5, 70, 5), extend='both', origin="lower")


def _hatching_wave():
    x = np.linspace(-3, 5, 150).reshape(1, -1)
    y = np.linspace(-3, 5, 120).reshape(-1, 1)
    return x.flatten(), y.flatten(), np.cos(x) + np.sin(y)


@case("gallery_images/contourf_hatching_1")
def _(fig):
    x, y, z = _hatching_wave()
    ax1 = fig.subplots()
    cs = ax1.contourf(x, y, z, hatches=['-', '/', '\\', '//'], cmap='gray', extend='both',
                      alpha=0.5)
    fig.colorbar(cs)


@case("gallery_images/contourf_hatching_2")
def _(fig):
    x, y, z = _hatching_wave()
    ax2 = fig.subplots()
    ax2.contour(x, y, z, 6, colors='black', linestyles='-')
    cs = ax2.contourf(x, y, z, 6, colors='none', hatches=['.', '/', '\\', None, r'\\', '*'],
                      extend='lower')
    artists, labels = cs.legend_elements(str_format='{:2.1f}'.format)
    ax2.legend(artists, labels, handleheight=2, framealpha=1)


@case("gallery_images/contourf_log")
def _(fig):
    X, Y = np.meshgrid(np.linspace(-3.0, 3.0, 100), np.linspace(-2.0, 2.0, 100))
    z = _spike(X, Y)
    z[:5, :5] = -1
    z = np.ma.masked_where(z <= 0, z)
    ax = fig.subplots()
    cs = ax.contourf(X, Y, z, locator=ticker.LogLocator(), cmap="PuBu_r")
    fig.colorbar(cs)


@case("gallery_images/contour_image")
def _(fig):
    extent = (-3, 4, -4, 3)
    X, Y = np.meshgrid(np.arange(-3.0, 4.001, 0.5), np.arange(-4.0, 3.001, 0.5))
    Z = _two_bumps(X, Y)
    levels = np.arange(-2.0, 1.601, 0.4)
    norm = colors.Normalize(vmax=abs(Z).max(), vmin=-abs(Z).max())
    cmap = plt.colormaps["PRGn"]
    _axs = fig.subplots(nrows=2, ncols=2)
    fig.subplots_adjust(hspace=0.3)
    axs = _axs.flatten()
    cset1 = axs[0].contourf(X, Y, Z, levels, norm=norm, cmap=cmap.resampled(len(levels) - 1))
    cset2 = axs[0].contour(X, Y, Z, cset1.levels, colors='k')
    cset2.set_linestyle('solid')
    axs[0].contour(X, Y, Z, (0,), colors='g', linewidths=2)
    axs[0].set_title('Filled contours')
    fig.colorbar(cset1, ax=axs[0])
    axs[1].imshow(Z, extent=extent, cmap=cmap, norm=norm)
    axs[1].contour(Z, levels, colors='k', origin='upper', extent=extent)
    axs[1].set_title("Image, origin 'upper'")
    axs[2].imshow(Z, origin='lower', extent=extent, cmap=cmap, norm=norm)
    axs[2].contour(Z, levels, colors='k', origin='lower', extent=extent)
    axs[2].set_title("Image, origin 'lower'")
    im = axs[3].imshow(Z, interpolation='nearest', extent=extent, cmap=cmap, norm=norm)
    axs[3].contour(Z, levels, colors='k', origin='image', extent=extent)
    ylim = axs[3].get_ylim()
    axs[3].set_ylim(ylim[::-1])
    axs[3].set_title("Origin from rc, reversed y-axis")
    fig.colorbar(im, ax=axs[3])
    fig.tight_layout()


def _percent(x):
    s = f"{x:.1f}"
    if s.endswith("0"):
        s = f"{x:.0f}"
    return f"{s} %"


@case("gallery_images/contour_label_demo_1")
def _(fig):
    X, Y, Z = _contour_field()
    ax = fig.subplots()
    CS = ax.contour(X, Y, Z)
    ax.clabel(CS, CS.levels, fmt=_percent, fontsize=10)


@case("gallery_images/contour_label_demo_2")
def _(fig):
    X, Y, Z = _contour_field()
    ax1 = fig.subplots()
    CS1 = ax1.contour(X, Y, Z)
    strs = ['first', 'second', 'third', 'fourth', 'fifth', 'sixth', 'seventh']
    ax1.clabel(CS1, CS1.levels[::2], fmt=dict(zip(CS1.levels, strs)), fontsize=10)


@case("gallery_images/contour_label_demo_3")
def _(fig):
    X, Y, Z = _contour_field()
    ax2 = fig.subplots()
    CS2 = ax2.contour(X, Y, 100**Z, locator=ticker.LogLocator())
    fmt = ticker.LogFormatterMathtext()
    fmt.create_dummy_axis()
    ax2.clabel(CS2, CS2.levels, fmt=fmt)
    ax2.set_title("$100^Z$")


@case("gallery_images/contours_in_optimization_demo", figsize=(6, 6))
def _(fig):
    ax = fig.subplots()
    x1, x2 = np.meshgrid(np.linspace(0.001, 4.0, 101), np.linspace(0.001, 4.0, 105))
    obj = x1**2 + x2**2 - 2 * x1 - 2 * x2 + 2
    g1 = -(3 * x1 + x2 - 5.5)
    g2 = -(x1 + 2 * x2 - 4.5)
    g3 = 0.8 + x1**-3 - x2
    cntr = ax.contour(x1, x2, obj, [0.01, 0.1, 0.5, 1, 2, 4, 8, 16], colors='black')
    ax.clabel(cntr, fmt="%2.1f", use_clabeltext=True)
    cg1 = ax.contour(x1, x2, g1, [0], colors='sandybrown')
    cg1.set(path_effects=[patheffects.withTickedStroke(angle=135)])
    cg2 = ax.contour(x1, x2, g2, [0], colors='orangered')
    cg2.set(path_effects=[patheffects.withTickedStroke(angle=60, length=2)])
    cg3 = ax.contour(x1, x2, g3, [0], colors='mediumblue')
    cg3.set(path_effects=[patheffects.withTickedStroke(spacing=7)])
    ax.set_xlim(0, 4)
    ax.set_ylim(0, 4)


@case("gallery_images/irregulardatagrid")
def _(fig):
    g = rng(SEED)
    npts = 200
    ngridx = 100
    ngridy = 200
    x = g.uniform(npts, -2, 2)
    y = g.uniform(npts, -2, 2)
    # Whether contourpy closes one of the loops depends on the last bits.
    z = x * _libm_exp(-x**2 - y**2)
    ax1, ax2 = fig.subplots(nrows=2)
    xi = np.linspace(-2.1, 2.1, ngridx)
    yi = np.linspace(-2.1, 2.1, ngridy)
    interpolator = tri.LinearTriInterpolator(tri.Triangulation(x, y), z)
    Xi, Yi = np.meshgrid(xi, yi)
    zi = interpolator(Xi, Yi)
    ax1.contour(xi, yi, zi, levels=14, linewidths=0.5, colors='k')
    cntr1 = ax1.contourf(xi, yi, zi, levels=14, cmap="RdBu_r")
    fig.colorbar(cntr1, ax=ax1)
    ax1.plot(x, y, 'ko', ms=3)
    ax1.set(xlim=(-2, 2), ylim=(-2, 2))
    ax1.set_title('grid and contour (%d points, %d grid points)' % (npts, ngridx * ngridy))
    ax2.tricontour(x, y, z, levels=14, linewidths=0.5, colors='k')
    cntr2 = ax2.tricontourf(x, y, z, levels=14, cmap="RdBu_r")
    fig.colorbar(cntr2, ax=ax2)
    ax2.plot(x, y, 'ko', ms=3)
    ax2.set(xlim=(-2, 2), ylim=(-2, 2))
    ax2.set_title('tricontour (%d points)' % npts)
    fig.subplots_adjust(hspace=0.5)


# --- triangular grids ------------------------------------------------------

def _rings(n_angles):
    """The triangulation and field of ``tricontour_demo`` (48 angles),
    ``tripcolor_demo`` and ``triplot_demo`` (36 angles)."""
    x, y, radii, angles = _annulus(n_angles, 8, 0.25)
    z = (np.cos(radii) * np.cos(3 * angles)).flatten()
    return _masked_delaunay(x, y, 0.25), z


@case("gallery_images/tricontour_demo_1")
def _(fig):
    triang, z = _rings(48)
    ax1 = fig.subplots()
    ax1.set_aspect('equal')
    tcf = ax1.tricontourf(triang, z)
    fig.colorbar(tcf)
    ax1.tricontour(triang, z, colors='k')
    ax1.set_title('Contour plot of Delaunay triangulation')


@case("gallery_images/tricontour_demo_2")
def _(fig):
    triang, z = _rings(48)
    ax2 = fig.subplots()
    ax2.set_aspect("equal")
    tcf = ax2.tricontourf(triang, z, hatches=["*", "-", "/", "//", "\\", None], cmap="cividis")
    fig.colorbar(tcf)
    ax2.tricontour(triang, z, linestyles="solid", colors="k", linewidths=2.0)
    ax2.set_title("Hatched Contour plot of Delaunay triangulation")


@case("gallery_images/tricontour_demo_3")
def _(fig):
    triang, z = _rings(48)
    ax3 = fig.subplots()
    tcf = ax3.tricontourf(triang, z, 7, colors="none",
                          hatches=[".", "/", "\\", None, "\\\\", "*"])
    ax3.tricontour(triang, z, 7, colors="black", linestyles="-")
    artists, labels = tcf.legend_elements(str_format="{:2.1f}".format)
    ax3.legend(artists, labels, handleheight=2, framealpha=1)


@case("gallery_images/tricontour_demo_4")
def _(fig):
    x, y = _britain()
    z = _wales_bump(x, y)
    ax4 = fig.subplots()
    ax4.set_aspect('equal')
    tcf = ax4.tricontourf(x, y, BRITAIN_TRIANGLES, z)
    fig.colorbar(tcf)
    _britain_labels(ax4, 'Contour plot of user-specified triangulation')


@case("gallery_images/tricontour_smooth_delaunay")
def _(fig):
    def experiment_res(x, y):
        x = 2 * x
        r1 = np.sqrt((0.5 - x)**2 + (0.5 - y)**2)
        theta1 = np.arctan2(0.5 - x, 0.5 - y)
        r2 = np.sqrt((-x - 0.2)**2 + (-y - 0.2)**2)
        theta2 = np.arctan2(-x - 0.2, -y - 0.2)
        z = (4 * (np.exp((r1 / 10)**2) - 1) * 30 * np.cos(3 * theta1) +
             (np.exp((r2 / 10)**2) - 1) * 30 * np.cos(5 * theta2) +
             2 * (x**2 + y**2))
        return (np.max(z) - z) / (np.max(z) - np.min(z))

    g = rng(SEED)
    x_test = g.uniform(200, -1., 1.)
    y_test = g.uniform(200, -1., 1.)
    z_test = experiment_res(x_test, y_test)
    triangulation = tri.Triangulation(x_test, y_test)
    mask = tri.TriAnalyzer(triangulation).get_flat_tri_mask(.01)
    triangulation.set_mask(mask)
    tri_refi, z_test_refi = tri.UniformTriRefiner(triangulation).refine_field(z_test, subdiv=3)
    flat_tri = tri.Triangulation(x_test, y_test)
    flat_tri.set_mask(~mask)
    ax = fig.subplots()
    ax.set_aspect('equal')
    ax.set_title("Filtering a Delaunay mesh\n(application to high-resolution tricontouring)")
    ax.tricontour(tri_refi, z_test_refi, levels=np.arange(0., 1., 0.025), cmap='Blues',
                  linewidths=[2.0, 0.5, 1.0, 0.5])
    ax.triplot(triangulation, color='0.7')
    ax.triplot(flat_tri, color='red')


@case("gallery_images/tricontour_smooth_user")
def _(fig):
    def function_z(x, y):
        r1 = np.sqrt((0.5 - x)**2 + (0.5 - y)**2)
        theta1 = np.arctan2(0.5 - x, 0.5 - y)
        r2 = np.sqrt((-x - 0.2)**2 + (-y - 0.2)**2)
        theta2 = np.arctan2(-x - 0.2, -y - 0.2)
        z = -(2 * (np.exp((r1 / 10)**2) - 1) * 30. * np.cos(7. * theta1) +
              (np.exp((r2 / 10)**2) - 1) * 30. * np.cos(11. * theta2) +
              0.7 * (x**2 + y**2))
        return (np.max(z) - z) / (np.max(z) - np.min(z))

    x, y, _, _ = _annulus(20, 10, 0.15)
    triang = _masked_delaunay(x, y, 0.15)
    tri_refi, z_test_refi = tri.UniformTriRefiner(triang).refine_field(function_z(x, y),
                                                                        subdiv=3)
    ax = fig.subplots()
    ax.set_aspect('equal')
    ax.triplot(triang, lw=0.5, color='white')
    levels = np.arange(0., 1., 0.025)
    ax.tricontourf(tri_refi, z_test_refi, levels=levels, cmap='terrain')
    ax.tricontour(tri_refi, z_test_refi, levels=levels,
                  colors=['0.25', '0.5', '0.5', '0.5', '0.5'],
                  linewidths=[1.0, 0.5, 0.5, 0.5, 0.5])
    ax.set_title("High-resolution tricontouring")


@case("gallery_images/trigradient_demo")
def _(fig):
    def dipole_potential(x, y):
        r_sq = x**2 + y**2
        theta = np.arctan2(y, x)
        z = np.cos(theta) / r_sq
        return (np.max(z) - z) / (np.max(z) - np.min(z))

    x, y, _, _ = _annulus(30, 10, 0.2)
    V = dipole_potential(x, y)
    triang = _masked_delaunay(x, y, 0.2)
    tri_refi, z_test_refi = tri.UniformTriRefiner(triang).refine_field(V, subdiv=3)
    tci = tri.CubicTriInterpolator(triang, -V)
    (Ex, Ey) = tci.gradient(triang.x, triang.y)
    E_norm = np.sqrt(Ex**2 + Ey**2)
    ax = fig.subplots()
    ax.set_aspect('equal')
    ax.use_sticky_edges = False
    ax.margins(0.07)
    ax.triplot(triang, color='0.8')
    ax.tricontour(tri_refi, z_test_refi, levels=np.arange(0., 1., 0.01), cmap='hot',
                  linewidths=[2.0, 1.0, 1.0, 1.0])
    ax.quiver(triang.x, triang.y, Ex / E_norm, Ey / E_norm, units='xy', scale=10., zorder=3,
              color='blue', width=0.007, headwidth=3., headlength=4.)
    ax.set_title('Gradient plot: an electrical dipole')


@case("gallery_images/triinterp_demo")
def _(fig):
    x = np.asarray([0, 1, 2, 3, 0.5, 1.5, 2.5, 1, 2, 1.5])
    y = np.asarray([0, 0, 0, 0, 1.0, 1.0, 1.0, 2, 2, 3.0])
    triangles = [[0, 1, 4], [1, 2, 5], [2, 3, 6], [1, 5, 4], [2, 6, 5], [4, 5, 7],
                 [5, 6, 8], [5, 8, 7], [7, 8, 9]]
    triang = tri.Triangulation(x, y, triangles)
    z = np.cos(1.5 * x) * np.cos(1.5 * y)
    xi, yi = np.meshgrid(np.linspace(0, 3, 20), np.linspace(0, 3, 20))
    axs = fig.subplots(nrows=2, ncols=2).flatten()
    axs[0].tricontourf(triang, z)
    axs[0].triplot(triang, 'ko-')
    axs[0].set_title('Triangular grid')
    for ax, interpolator, title in [
        (axs[1], tri.LinearTriInterpolator(triang, z), "Linear interpolation"),
        (axs[2], tri.CubicTriInterpolator(triang, z, kind='geom'),
         "Cubic interpolation,\nkind='geom'"),
        (axs[3], tri.CubicTriInterpolator(triang, z, kind='min_E'),
         "Cubic interpolation,\nkind='min_E'"),
    ]:
        ax.contourf(xi, yi, interpolator(xi, yi))
        ax.plot(xi, yi, 'k-', lw=0.5, alpha=0.5)
        ax.plot(xi.T, yi.T, 'k-', lw=0.5, alpha=0.5)
        ax.set_title(title)
    fig.tight_layout()


@case("gallery_images/tripcolor_demo_1")
def _(fig):
    triang, z = _rings(36)
    ax1 = fig.subplots()
    ax1.set_aspect('equal')
    tpc = ax1.tripcolor(triang, z, shading='flat')
    fig.colorbar(tpc)
    ax1.set_title('tripcolor of Delaunay triangulation, flat shading')


@case("gallery_images/tripcolor_demo_2")
def _(fig):
    triang, z = _rings(36)
    ax2 = fig.subplots()
    ax2.set_aspect('equal')
    tpc = ax2.tripcolor(triang, z, shading='gouraud')
    fig.colorbar(tpc)
    ax2.set_title('tripcolor of Delaunay triangulation, gouraud shading')


@case("gallery_images/tripcolor_demo_3")
def _(fig):
    x, y = _britain()
    xmid = x[BRITAIN_TRIANGLES].mean(axis=1)
    ymid = y[BRITAIN_TRIANGLES].mean(axis=1)
    zfaces = _wales_bump(xmid, ymid)
    ax3 = fig.subplots()
    ax3.set_aspect('equal')
    tpc = ax3.tripcolor(x, y, BRITAIN_TRIANGLES, facecolors=zfaces, edgecolors='k')
    fig.colorbar(tpc)
    _britain_labels(ax3, 'tripcolor of user-specified triangulation')


@case("gallery_images/triplot_demo_1")
def _(fig):
    triang, _ = _rings(36)
    ax1 = fig.subplots()
    ax1.set_aspect('equal')
    ax1.triplot(triang, 'bo-', lw=1)
    ax1.set_title('triplot of Delaunay triangulation')


@case("gallery_images/triplot_demo_2")
def _(fig):
    x, y = _britain()
    ax2 = fig.subplots()
    ax2.set_aspect('equal')
    ax2.triplot(x, y, BRITAIN_TRIANGLES, 'go-', lw=1.0)
    _britain_labels(ax2, 'triplot of user-specified triangulation')


# --- vector fields ---------------------------------------------------------

def _quiver_field():
    X, Y = np.meshgrid(np.arange(0, 2 * np.pi, .2), np.arange(0, 2 * np.pi, .2))
    return X, Y, np.cos(X), np.sin(Y)


@case("gallery_images/quiver_demo_1")
def _(fig):
    X, Y, U, V = _quiver_field()
    ax1 = fig.subplots()
    ax1.set_title('Arrows scale with plot width, not view')
    Q = ax1.quiver(X, Y, U, V, units='width')
    ax1.quiverkey(Q, 0.9, 0.9, 2, r'$2 \frac{m}{s}$', labelpos='E', coordinates='figure')


@case("gallery_images/quiver_demo_2")
def _(fig):
    X, Y, U, V = _quiver_field()
    ax2 = fig.subplots()
    ax2.set_title("pivot='middle'; every third arrow; units='inches'")
    Q = ax2.quiver(X[::3, ::3], Y[::3, ::3], U[::3, ::3], V[::3, ::3], pivot='middle',
                   units='inches')
    ax2.quiverkey(Q, 0.9, 0.9, 1, r'$1 \frac{m}{s}$', labelpos='E', coordinates='figure')
    ax2.scatter(X[::3, ::3], Y[::3, ::3], color='r', s=5)


@case("gallery_images/quiver_demo_3")
def _(fig):
    X, Y, U, V = _quiver_field()
    ax3 = fig.subplots()
    ax3.set_title("pivot='tip'; scales with x view")
    M = np.hypot(U, V)
    Q = ax3.quiver(X, Y, U, V, M, units='x', pivot='tip', width=0.022, scale=1 / 0.15)
    ax3.quiverkey(Q, 0.9, 0.9, 1, r'$1 \frac{m}{s}$', labelpos='E', coordinates='figure')
    ax3.scatter(X, Y, color='0.5', s=1)


@case("gallery_images/quiver_simple_demo")
def _(fig):
    X = np.arange(-10, 10, 1)
    Y = np.arange(-10, 10, 1)
    U, V = np.meshgrid(X, Y)
    ax = fig.subplots()
    q = ax.quiver(X, Y, U, V)
    ax.quiverkey(q, X=0.3, Y=1.1, U=10, label='Quiver key, length = 10', labelpos='E')


@case("gallery_images/plot_streamplot_1", figsize=(7, 12))
def _(fig):
    w = 3
    Y, X = np.mgrid[-w:w:100j, -w:w:100j]
    U = -1 - X**2 + Y
    V = 1 + X - Y**2
    speed = np.sqrt(U**2 + V**2)
    axs = fig.subplots(4, 2, height_ratios=[1, 1, 1, 2]).flat
    axs[0].streamplot(X, Y, U, V, density=[0.5, 1])
    axs[0].set_title('Varying Density')
    strm = axs[1].streamplot(X, Y, U, V, color=U, linewidth=2, cmap='autumn')
    fig.colorbar(strm.lines)
    axs[1].set_title('Varying Color')
    lw = 5 * speed / speed.max()
    axs[2].streamplot(X, Y, U, V, density=0.6, color='k', linewidth=lw, num_arrows=5)
    axs[2].set_title('Varying Line Width')
    seed_points = np.array([[-2, -1, 0, 1, 2, -1], [-2, -1, 0, 1, 2, 2]])
    strm = axs[3].streamplot(X, Y, U, V, color=U, linewidth=2, cmap='autumn',
                             start_points=seed_points.T)
    fig.colorbar(strm.lines)
    axs[3].set_title('Controlling Starting Points')
    axs[3].plot(seed_points[0], seed_points[1], 'bo')
    axs[3].set(xlim=(-w, w), ylim=(-w, w))
    axs[4].streamplot(X, Y, U, V, num_arrows=3)
    axs[4].set_title('Multiple arrows')
    axs[5].axis("off")
    mask = np.zeros(U.shape, dtype=bool)
    mask[40:60, 40:60] = True
    U[:20, :20] = np.nan
    U = np.ma.array(U, mask=mask)
    axs[6].streamplot(X, Y, U, V, color='r')
    axs[6].set_title('Streamplot with Masking')
    axs[6].imshow(~mask, extent=(-w, w, -w, w), alpha=0.5, cmap='gray', aspect='auto')
    axs[6].set_aspect('equal')
    axs[7].streamplot(X, Y, U, V, broken_streamlines=False)
    axs[7].set_title('Streamplot with unbroken streamlines')
    fig.tight_layout()


@case("gallery_images/plot_streamplot_2", figsize=(6, 14))
def _(fig):
    n = 50
    x, y = np.meshgrid(np.linspace(-2, 2, n), np.linspace(-3, 3, n))
    th = np.arctan2(y, x)
    r = np.sqrt(x**2 + y**2)
    vr = -np.cos(th) / r**2
    vt = -np.sin(th) / r**2 - 1 / r
    vx = vr * np.cos(th) - vt * np.sin(th) + 1.0
    vy = vr * np.sin(th) + vt * np.cos(th)
    seed_pts = np.column_stack((np.full(50, -1.75), np.linspace(-2, 2, 50)))
    axs = fig.subplots(3, 1)
    th_circ = np.linspace(0, 2 * np.pi, 100)
    for ax, max_val in zip(axs, [0.05, 1, 5]):
        ax_ins = ax.inset_axes([0.0, 0.7, 0.3, 0.35])
        for ax_curr, is_inset in zip([ax, ax_ins], [False, True]):
            ax_curr.streamplot(x, y, vx, vy, start_points=seed_pts, broken_streamlines=False,
                               arrowsize=1e-10, linewidth=2 if is_inset else 0.6, color="k",
                               integration_max_step_scale=max_val,
                               integration_max_error_scale=max_val)
            ax_curr.fill(np.cos(th_circ), np.sin(th_circ), color="w", ec="k",
                         lw=6 if is_inset else 2)
            ax_curr.set_aspect("equal")
        # The example also prints how long streamplot took, which varies.
        text = (f"integration_max_step_scale: {max_val}\n"
                f"integration_max_error_scale: {max_val}")
        if max_val == 1:
            text += "\n(default)"
        ax.text(0.0, 0.0, text, ha="center", va="center")
        ax_ins.set_xlim(-1.2, -0.7)
        ax_ins.set_ylim(-0.8, -0.4)
        ax_ins.set_yticks(())
        ax_ins.set_xticks(())
        ax.set_ylim(-1.5, 1.5)
        ax.axis("off")
        ax.indicate_inset_zoom(ax_ins, ec="k")
    fig.tight_layout()
