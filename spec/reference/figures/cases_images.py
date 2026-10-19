"""Images: imshow (interpolation, stages, origin/extent/aspect, RGB(A), alpha,
norms, extreme colours, masks), matshow, spy, figimage, pcolor, pcolormesh,
pcolorfast, NonUniformImage, hexbin, multivariate data and imread'd PNGs."""

import io

import matplotlib as mpl
import numpy as np
from matplotlib import colors, image, patches
from matplotlib.colorizer import _ensure_multivariate_data
from matplotlib.image import NonUniformImage
from PIL import Image

from spec_support import case, rng

INTERPOLATIONS = [
    "none", "nearest", "bilinear", "bicubic", "spline16", "spline36",
    "hanning", "hamming", "hermite", "kaiser", "quadric", "catrom",
    "gaussian", "bessel", "mitchell", "sinc", "lanczos", "blackman",
    "antialiased", "auto",
]


def _small():
    """8x8 test pattern: checkerboard quadrant, an isolated spike, a ramp."""
    a = np.zeros((8, 8))
    a[:4, :4] = np.indices((4, 4)).sum(axis=0) % 2
    a[6, 2] = 1.0
    a[:, 5:] = np.linspace(0.0, 1.0, 8)[:, None]
    return a


def _rings(n):
    """n x n concentric rings whose frequency rises towards the corners."""
    x = np.linspace(-1.0, 1.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(120.0 * (xx**2 + yy**2))


def _bump(n):
    """Smooth n x n field in [-1, 1]."""
    x = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.sin(xx) * np.cos(yy * 0.8) * np.exp(-(xx**2 + yy**2) / 12.0)


# --- every interpolation filter: small array upsampled (left) and a large
# --- array downsampled (right).

def _interp_case(interpolation):
    def build(fig):
        ax1, ax2 = fig.subplots(1, 2)
        ax1.imshow(_small(), interpolation=interpolation)
        ax2.imshow(_rings(400), interpolation=interpolation)

    return build


for _name in INTERPOLATIONS:
    case(f"images/interp_{_name}", figsize=(5, 2.5))(_interp_case(_name))


@case("images/interp_stage_data", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_small(), interpolation="bilinear", interpolation_stage="data", cmap="RdBu")
    ax2.imshow(_rings(400), interpolation="hanning", interpolation_stage="data", cmap="RdBu")


@case("images/interp_stage_rgba", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_small(), interpolation="bilinear", interpolation_stage="rgba", cmap="RdBu")
    ax2.imshow(_rings(400), interpolation="hanning", interpolation_stage="rgba", cmap="RdBu")


@case("images/filterrad_sinc", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_small(), interpolation="sinc", filterrad=2.0)
    ax2.imshow(_small(), interpolation="lanczos", filternorm=False)


# --- defaults, origin, extent, aspect

@case("images/imshow_default")
def _(fig):
    ax = fig.subplots()
    ax.imshow(rng(1).uniform(120).reshape(10, 12))


@case("images/origin_lower", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_small(), origin="lower")


@case("images/extent", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_bump(30), extent=(-3, 3, -2, 2))


@case("images/extent_origin_lower_and_reversed", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_small(), origin="lower", extent=(0, 16, 10, 30))
    ax2.imshow(_small(), extent=(8, 0, 0, 8))


@case("images/aspect_auto_and_numeric", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(_bump(20), aspect="auto")
    ax2.imshow(_small(), aspect=0.5)


@case("images/xlim_zoom", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_bump(40))
    ax.set_xlim(10, 25)
    ax.set_ylim(30, 5)


# --- RGB(A) inputs

def _rgb_float(n):
    x = np.linspace(0.0, 1.0, n)
    xx, yy = np.meshgrid(x, x)
    return np.dstack([xx, yy, 1.0 - xx * yy])


@case("images/rgb_float_uint8_rgba", figsize=(6, 2.5))
def _(fig):
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.imshow(_rgb_float(6))
    ax2.imshow((_rgb_float(6) * 255).astype(np.uint8))
    ax3.plot([0, 5], [0, 5], color="k", lw=4)
    ax3.imshow(np.dstack([_rgb_float(6), np.linspace(0.1, 1.0, 6)[None, :].repeat(6, axis=0)]))


@case("images/rgba_uint8_downsampled", figsize=(3, 3))
def _(fig):
    ax = fig.subplots()
    img = np.dstack([_rgb_float(300), (_rings(300) > 0) * 0.5 + 0.5])
    ax.imshow((img * 255).astype(np.uint8))


# --- alpha

@case("images/alpha_scalar_and_array", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    for ax in (ax1, ax2):
        ax.plot([-0.5, 7.5], [-0.5, 7.5], color="r", lw=6)
    ax1.imshow(_small(), alpha=0.5)
    ax2.imshow(_small(), alpha=np.linspace(0.0, 1.0, 8)[None, :].repeat(8, axis=0))


# --- normalisation and colour mapping

@case("images/vmin_vmax", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_bump(20), vmin=-0.3, vmax=0.3, cmap="coolwarm")


@case("images/lognorm", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 4.0, 40)
    ax.imshow(10.0 ** np.add.outer(x, x / 2), norm=colors.LogNorm())


@case("images/symlognorm", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(100 * _bump(30) ** 3, norm=colors.SymLogNorm(linthresh=0.5), cmap="RdBu_r")


@case("images/powernorm", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(np.abs(_bump(30)), norm=colors.PowerNorm(gamma=0.4), cmap="magma")


@case("images/boundarynorm", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    norm = colors.BoundaryNorm([-1.0, -0.5, -0.1, 0.1, 0.5, 1.0], ncolors=256)
    ax.imshow(_bump(30), norm=norm, cmap="RdBu")


@case("images/cmap_resampled", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_bump(30), cmap=mpl.colormaps["viridis"].resampled(5))


@case("images/cmap_extremes", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    data = np.ma.masked_greater(_bump(20), 0.6)
    data[3, 3] = np.nan
    cmap = mpl.colormaps["viridis"].with_extremes(bad="red", under="black", over="white")
    ax.imshow(data, cmap=cmap, vmin=-0.4, vmax=0.4)


@case("images/masked_default", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 19], [0, 19], color="k", lw=3)
    data = np.ma.masked_inside(_bump(20), -0.1, 0.1)
    ax.imshow(data)


# --- matshow, spy, figimage

@case("images/matshow", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.matshow(np.arange(35).reshape(5, 7) % 6)


@case("images/spy_image_and_markers", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    a = np.where(rng(2).uniform(400).reshape(20, 20) > 0.8, 1.0, 0.0)
    ax1.spy(a)
    ax2.spy(a, markersize=4, precision=0.5)


@case("images/figimage", figsize=(4, 3))
def _(fig):
    fig.figimage(_bump(80), xo=20, yo=30, cmap="viridis")
    fig.figimage((_rgb_float(100) * 255).astype(np.uint8), xo=70, yo=80, alpha=0.6)
    fig.figimage(_bump(50), xo=300, yo=200, origin="lower")


# --- pcolor / pcolormesh / pcolorfast

def _grid(nx, ny):
    x = np.linspace(-3.0, 3.0, nx)
    y = np.linspace(-2.0, 2.0, ny)
    return np.meshgrid(x, y)


@case("images/pcolor_default", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.pcolor(rng(3).uniform(48).reshape(6, 8))


@case("images/pcolor_edges_masked", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(9, 7)
    c = np.ma.masked_greater(np.sin(xx[:-1, :-1]) * np.cos(yy[:-1, :-1]), 0.7)
    ax.pcolor(xx, yy, c, edgecolors="k", linewidths=1.5, cmap="PuOr")


@case("images/pcolormesh_flat", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(13, 9)
    ax.pcolormesh(xx, yy, (xx * yy)[:-1, :-1], shading="flat")


@case("images/pcolormesh_nearest", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(12, 8)
    ax.pcolormesh(xx, yy, xx * yy, shading="nearest")


@case("images/pcolormesh_gouraud", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(6, 5)
    ax.pcolormesh(xx, yy, np.sin(xx) + np.cos(yy), shading="gouraud")


@case("images/pcolormesh_auto_same_shape", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.pcolormesh([1, 2, 4, 8], [0, 1, 3], np.arange(12).reshape(3, 4))


@case("images/pcolormesh_nonuniform", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.array([0.0, 0.5, 1.0, 2.0, 4.0, 7.0, 11.0])
    y = np.array([0.0, 1.0, 1.5, 3.5, 4.0])
    ax.pcolormesh(x, y, np.arange(24).reshape(4, 6), cmap="cividis")


@case("images/pcolormesh_curvilinear_edges", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    r, t = np.meshgrid(np.linspace(1.0, 2.0, 6), np.linspace(0.0, np.pi / 2, 10))
    xx, yy = r * np.cos(t), r * np.sin(t)
    ax.pcolormesh(xx, yy, (r * t)[:-1, :-1], edgecolors="white", linewidth=0.8)
    ax.set_aspect("equal")


@case("images/pcolormesh_alpha_log_axes", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x = np.geomspace(1.0, 1000.0, 8)
    y = np.linspace(0.0, 1.0, 5)
    ax.pcolormesh(x, y, np.arange(28).reshape(4, 7), alpha=0.6)
    ax.set_xscale("log")


@case("images/pcolorfast_uniform", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.pcolorfast((0, 10), (0, 5), _bump(12))


@case("images/pcolorfast_nonuniform", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.pcolorfast([0, 1, 3, 6, 10], [0, 2, 3, 5], np.arange(12).reshape(3, 4), cmap="plasma")


@case("images/pcolorfast_quadmesh", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(7, 6)
    xx = xx + 0.3 * yy
    ax.pcolorfast(xx, yy, (xx * yy)[:-1, :-1])



# --- NonUniformImage

def _nonuniform(ax, interpolation, x, y, data, **kwargs):
    im = NonUniformImage(ax, interpolation=interpolation, **kwargs)
    im.set_data(x, y, data)
    ax.add_image(im)


def _cubic_grid():
    """Centres crowded near 0 along x, even along y, and their radius."""
    x = np.linspace(-4.0, 4.0, 9) ** 3 / 16.0
    y = np.linspace(-4.0, 4.0, 7)
    return x, y, np.sqrt(x[None, :] ** 2 + y[:, None] ** 2)


@case("images/nonuniform_nearest", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    _nonuniform(ax, "nearest", *_cubic_grid(), cmap="viridis")
    ax.set(xlim=(-5, 5), ylim=(-5, 5))


@case("images/nonuniform_bilinear_relim", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    _nonuniform(ax, "bilinear", *_cubic_grid(), cmap="magma")
    ax.relim()
    ax.autoscale_view()


@case("images/nonuniform_rgb_log", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    _nonuniform(ax, "bilinear", [1.0, 3.0, 10.0, 30.0, 100.0], [0.0, 1.0, 3.0, 3.5, 6.0],
                _rgb_float(5))
    ax.set_xscale("log")
    ax.set(xlim=(0.5, 200.0), ylim=(-1.0, 7.0))


# --- hexbin

def _cloud(n, seed):
    g = rng(seed)
    x = g.normal(n)
    y = 0.6 * x + g.normal(n, scale=0.8)
    return x, y


@case("images/hexbin_default")
def _(fig):
    ax = fig.subplots()
    ax.hexbin(*_cloud(3000, 4))


@case("images/hexbin_gridsize_tuple", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.hexbin(*_cloud(2000, 4), gridsize=(15, 6), cmap="Blues", edgecolors="k", linewidths=0.3)


@case("images/hexbin_log", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.hexbin(*_cloud(3000, 4), gridsize=20, bins="log", cmap="inferno")


@case("images/hexbin_mincnt_extent", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.hexbin(*_cloud(2000, 4), gridsize=18, mincnt=3, extent=(-4, 4, -4, 4))


@case("images/hexbin_reduce_mean", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    x, y = _cloud(2000, 5)
    ax.hexbin(x, y, C=x * y, reduce_C_function=np.mean, gridsize=15, cmap="RdBu")


# --- clipping

@case("images/clip_path_circle", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_bump(40))
    circle = patches.Circle((20, 20), 15, transform=ax.transData)
    im.set_clip_path(circle)


# --- transforms, nonlinear axes, clipping and compositing

@case("images/log_axes_resampled", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    z = np.zeros((10, 10))
    z[::2] = 1
    ax1.imshow(z, extent=[1, 100, 1, 100], cmap="viridis", vmax=1, vmin=-1, aspect="auto")
    ax1.set(yscale="log")
    ax2.imshow(_bump(30), extent=[1, 1000, 0, 1], aspect="auto", interpolation="bilinear")
    ax2.set_xscale("log")


def _affine_image():
    x = np.arange(-3.0, 3.0, 0.25)
    xx, yy = np.meshgrid(x, x)
    return np.exp(-xx**2 - yy**2) - np.exp(-(xx - 1) ** 2 - (yy - 1) ** 2)


def _affine_plot(ax, transform):
    im = ax.imshow(_affine_image(), interpolation="none", origin="lower", extent=[-2, 4, -3, 2], clip_on=True)
    trans_data = transform + ax.transData
    im.set_transform(trans_data)
    x1, x2, y1, y2 = im.get_extent()
    ax.plot([x1, x2, x2, x1, x1], [y1, y1, y2, y2, y1], "y--", transform=trans_data)
    ax.set_xlim(-5, 5)
    ax.set_ylim(-4, 4)


@case("images/affine_transforms")
def _(fig):
    from matplotlib import transforms

    axs = fig.subplots(2, 2)
    _affine_plot(axs[0, 0], transforms.Affine2D().rotate_deg(30))
    _affine_plot(axs[0, 1], transforms.Affine2D().skew_deg(30, 15))
    _affine_plot(axs[1, 0], transforms.Affine2D().scale(-1, 0.5))
    _affine_plot(axs[1, 1], transforms.Affine2D().rotate_deg(30).skew_deg(30, 15).scale(-1, 0.5).translate(0.5, -1))


def _with_holes():
    data = _bump(12)
    data[2:4, 3:7] = np.nan
    data[8, 8] = np.inf
    data[9, 2] = -np.inf
    return data


@case("images/nan_inf_interpolation_stages", figsize=(6, 2.5))
def _(fig):
    axs = fig.subplots(1, 3)
    cmap = mpl.colormaps["viridis"].with_extremes(bad="magenta", under="k", over="w")
    axs[0].imshow(_with_holes(), cmap=cmap, interpolation="nearest")
    axs[1].imshow(_with_holes(), cmap=cmap, interpolation="bilinear", interpolation_stage="data")
    axs[2].imshow(_with_holes(), cmap=cmap, interpolation="bilinear", interpolation_stage="rgba")


@case("images/resample_factors", figsize=(6, 2.2))
def _(fig):
    axs = fig.subplots(1, 3)
    axs[0].imshow(_rings(60))
    axs[1].imshow(_small())
    axs[2].imshow(_rings(250), interpolation_stage="auto")
    axs[2].set_xlim(0, 120)


@case("images/origin_extent_grid", figsize=(6, 4))
def _(fig):
    axs = fig.subplots(2, 3)
    data = np.arange(12).reshape(3, 4)
    for row, origin in enumerate(["upper", "lower"]):
        for col, extent in enumerate([None, (0, 10, 0, 6), (10, 0, 6, 0)]):
            ax = axs[row, col]
            ax.imshow(data, origin=origin, extent=extent)
            ax.set_title(f"{origin}, {extent}", fontsize=8)


@case("images/clip_off_beyond_axes", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_bump(20), extent=(-1, 21, -1, 21), clip_on=False, origin="lower")
    ax.set_xlim(2, 18)
    ax.set_ylim(2, 18)
    fig.subplots_adjust(left=0.2, right=0.8, bottom=0.2, top=0.8)


@case("images/clip_path_rotated_rectangle", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(_bump(40), cmap="plasma")
    rect = patches.Rectangle((8, 8), 24, 12, angle=30, transform=ax.transData)
    im.set_clip_path(rect)


@case("images/composite_alpha", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    arr = np.zeros((11, 21, 4))
    arr[:, :, 0] = 1
    arr[:, :, 3] = np.concatenate((np.arange(0, 1.1, 0.1), np.arange(0, 1, 0.1)[::-1]))
    arr2 = np.zeros((21, 11, 4))
    arr2[:, :, 0] = 1
    arr2[:, :, 1] = 1
    arr2[:, :, 3] = np.concatenate((np.arange(0, 1.1, 0.1), np.arange(0, 1, 0.1)[::-1]))[:, np.newaxis]
    ax.imshow(arr, extent=[1, 2, 5, 0], alpha=0.3)
    ax.imshow(arr, extent=[2, 3, 5, 0], alpha=0.6)
    ax.imshow(arr, extent=[3, 4, 5, 0])
    ax.imshow(arr2, extent=[0, 5, 1, 2])
    ax.imshow(arr2, extent=[0, 5, 2, 3], alpha=0.6)
    ax.imshow(arr2, extent=[0, 5, 3, 4], alpha=0.3)
    ax.set_facecolor((0, 0.5, 0, 1))
    ax.set_xlim(0, 5)
    ax.set_ylim(5, 0)


@case("images/bool_and_integer_arrays", figsize=(6, 2.5))
def _(fig):
    axs = fig.subplots(1, 3)
    axs[0].imshow(_small() > 0.5)
    axs[1].imshow((np.arange(48).reshape(6, 8) * 37 % 101).astype(np.int16), cmap="tab20")
    axs[2].imshow((_small() * 60000).astype(np.uint16), cmap="gray", vmin=0, vmax=65535)


@case("images/pcolorfast_nonuniform_alpha", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 10], [0, 5], color="k", lw=4)
    ax.pcolorfast([0, 1, 3, 6, 10], [0, 2, 3, 5], np.arange(12).reshape(3, 4), cmap="plasma", alpha=0.5)


# --- multivariate data: matplotlib 3.11 maps it in pcolormesh and in
# --- quad-mesh pcolorfast (images reject it)

def _variates(*parts):
    """The multivariate array of equally shaped ``parts``."""
    return _ensure_multivariate_data(tuple(parts), len(parts))


def _variate_fields():
    """Three smooth 8 x 12 fields: a wave, a positive exponential, a saddle."""
    xx, yy = _grid(12, 8)
    return np.sin(xx) * np.cos(yy), np.exp(0.5 * (xx + yy)), xx * yy / 6.0


@case("images/pcolormesh_bivariate", figsize=(6, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    a, b, _ = _variate_fields()
    ax1.pcolormesh(_variates(a, b), cmap="BiPeak")
    xx, yy = _grid(13, 9)
    ax2.pcolormesh(xx, yy, _variates(a, b), cmap="BiOrangeBlue", norm=["linear", "log"],
                   vmin=[-0.5, None], vmax=[0.5, None])


@case("images/pcolormesh_bicone_masked_alpha", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([-3, 3], [-2, 2], color="k", lw=4)
    a, _, c = _variate_fields()
    a = np.where(np.arange(96).reshape(8, 12) % 7 == 0, np.nan, a)
    ax.pcolormesh(np.linspace(-3, 3, 12), np.linspace(-2, 2, 8), _variates(a, c), cmap="BiCone",
                  shading="nearest", alpha=0.6, norm=[colors.Normalize(vmin=-1, vmax=1), "linear"])


@case("images/pcolormesh_multivariate", figsize=(6, 2.2))
def _(fig):
    axs = fig.subplots(1, 3)
    a, b, c = _variate_fields()
    axs[0].pcolormesh(_variates(a, c), cmap="2VarAddA")
    axs[1].pcolormesh(_variates(b, a), cmap="2VarSubA", norm=[colors.PowerNorm(gamma=0.5), "linear"])
    axs[2].pcolormesh(_variates(a, b, c), cmap="3VarAddA", norm=["linear", "log", "linear"],
                      vmin=[-0.5, None, None], vmax=[0.5, None, 0.5])


@case("images/pcolorfast_multivariate_quadmesh", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(13, 9)
    a, b, c = _variate_fields()
    ax.pcolorfast(xx + 0.3 * yy, yy, _variates(a, b, c), cmap="3VarAddA",
                  norm=["linear", "symlog", "asinh"])


# --- PNGs read back with imread: float32 colours and greyscale

def _png_roundtrip(array):
    """``array`` written by ``imsave`` and read back by ``imread``."""
    buffer = io.BytesIO()
    image.imsave(buffer, array, format="png")
    buffer.seek(0)
    return image.imread(buffer)


def _gray_png_roundtrip(levels):
    """8-bit grey ``levels`` written as a greyscale PNG and read back by ``imread``."""
    buffer = io.BytesIO()
    Image.fromarray(levels).save(buffer, format="png")
    buffer.seek(0)
    return image.imread(buffer)


@case("images/imread_rgba_interpolation", figsize=(6, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    r, c = np.mgrid[0:12, 0:16]
    rgba = np.stack([(37 * r + 11 * c) % 256, 255 - 15 * c, 20 * r, (40 + 13 * r + 9 * c) % 256], axis=-1)
    img = _png_roundtrip(rgba.astype(np.uint8))
    for ax, interpolation in [(ax1, "nearest"), (ax2, "bilinear")]:
        ax.plot([0, 15], [0, 11], color="k", lw=5)
        ax.imshow(img, interpolation=interpolation)


@case("images/imread_gray_float32", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    r, c = np.mgrid[0:10, 0:12]
    gray = _gray_png_roundtrip(((19 * r + 23 * c) % 256).astype(np.uint8))
    ax1.imshow(gray, cmap="gray", vmin=0.1, vmax=0.9, interpolation="bilinear")
    ax2.imshow(gray, interpolation="nearest")
