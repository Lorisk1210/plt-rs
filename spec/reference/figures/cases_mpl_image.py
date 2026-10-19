"""matplotlib's own image tests from ``tests/test_image.py``: every
``@image_comparison`` test and the ``fig_test`` side of every
``@check_figures_equal`` test.

Each case keeps the test's style. The test suite's default (also used by
``check_figures_equal``) is ``classic`` plus ``_classic_test_patch``: such
figures are 8x6 in and saved at classic's 100 dpi. Tests asking for
``mpl20``/``default`` run in the reference configuration. A ``savefig.dpi``
the test sets becomes the case's dpi, ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and parametrized tests
contribute one case per parameter set.

Deviations from the tests:

* ``np.random`` draws (seeded or not) come from ``spec_support.rng``, seeded
  with the test's seed where it has one.
* The ``nonaffine_identity`` fixture is an identity transform matplotlib
  must treat as non-affine (:class:`NonAffineIdentity`).
* ``test_imshow_pil`` (PIL images of files from matplotlib's baseline
  directory) and ``test_large_image`` (``high_memory``: arrays of 2**23 and
  2**24 pixels) are not ported.
"""

import matplotlib
import matplotlib.style
import numpy as np
from matplotlib import colors as mcolors
from matplotlib import patches as mpatches
from matplotlib import path as mpath
from matplotlib import ticker as mticker
from matplotlib import transforms as mtransforms
from matplotlib.image import BboxImage, NonUniformImage
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case, rng

GROUP = "mpl_image"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def image_test(name, *, classic=False, figsize=None, dpi=100, remove_text=False,
               rc=None):
    """Register ``build`` as ``mpl_image/<name>`` under the conditions of
    matplotlib's test decorators (see the module docstring)."""
    params = {**(CLASSIC if classic else {}), **(rc or {})}
    default_size = (8, 6) if classic else (6.4, 4.8)

    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", figsize=figsize or default_size, dpi=dpi,
             rc=params)(run)
        return build

    return register


def classic(name, **options):
    """A test in the test suite's default style."""
    return image_test(name, classic=True, **options)


class NonAffineIdentity(mtransforms.Transform):
    """The ``nonaffine_identity`` fixture: the identity, as a transform
    matplotlib must treat as non-affine."""

    input_dims = 2
    output_dims = 2

    def inverted(self):
        return self


def _uniform(seed, *shape):
    """``np.random.rand(*shape)`` after ``np.random.seed(seed)``."""
    return rng(seed).uniform(int(np.prod(shape))).reshape(shape)


def _gaussians(delta, scale=1.0):
    """The difference of Gaussians of the rotate and mask tests."""
    x = y = np.arange(-3.0, 3.0, delta)
    X, Y = np.meshgrid(x, y)
    Z1 = np.exp(-(X**2 + Y**2) / 2) / (2 * np.pi)
    Z2 = (np.exp(-(((X - 1) / 1.5)**2 + ((Y - 1) / 0.5)**2) / 2) /
          (2 * np.pi * 0.5 * 1.5))
    return scale * (Z2 - Z1)


def _rings(n, f0, k):
    """The striped and ringing array of the antialiasing tests: alternating
    rows of -1 and 1, and aliased concentric circles with a striped disc."""
    x = np.arange(n) / n - 0.5
    X, Y = np.meshgrid(x, x)
    R = np.sqrt(X**2 + Y**2)
    a = np.sin(np.pi * 2 * (f0 * R + k * R**2 / 2))
    a[:int(n / 2), :][R[:int(n / 2), :] < 0.4] = -1
    a[:int(n / 2), :][R[:int(n / 2), :] < 0.3] = 1
    aa = np.ones((n, n))
    aa[::2, :] = -1
    return aa, a


# --- interpolation and alpha -------------------------------------------------


@classic("alpha_interp", remove_text=True)
def _(fig):
    axl, axr = fig.subplots(1, 2)
    img = np.zeros((5, 5, 4))
    img[..., 1] = np.ones((5, 5))
    img[..., 3] = np.tril(np.ones((5, 5), dtype=np.uint8))
    axl.imshow(img, interpolation="none")
    axr.imshow(img, interpolation="bilinear")


@image_test("interp_nearest_vs_none", dpi=3, remove_text=True)
def _(fig):
    X = np.array([[[218, 165, 32], [122, 103, 238]],
                  [[127, 255, 0], [255, 99, 71]]], dtype=np.uint8)
    ax1, ax2 = fig.subplots(1, 2)
    ax1.imshow(X, interpolation="none")
    ax1.set_title("interpolation none")
    ax2.imshow(X, interpolation="nearest")
    ax2.set_title("interpolation nearest")


def _figimage(fig, suppress_composite):
    fig.suppressComposite = suppress_composite
    x, y = np.ix_(np.arange(100) / 100.0, np.arange(100) / 100)
    z = np.sin(x**2 + y**2 - x * y)
    c = np.sin(20 * x**2 + 50 * y**2)
    img = z + c / 5
    fig.figimage(img, xo=0, yo=0, origin="lower")
    fig.figimage(img[::-1, :], xo=0, yo=100, origin="lower")
    fig.figimage(img[:, ::-1], xo=100, yo=0, origin="lower")
    fig.figimage(img[::-1, ::-1], xo=100, yo=100, origin="lower")


@classic("figimage", figsize=(2, 2))
def _(fig):
    _figimage(fig, False)


@classic("figimage_suppress_composite", figsize=(2, 2))
def _(fig):
    _figimage(fig, True)


#: ``test_imshow_antialiased``'s parameters: image size (in units of the
#: 100 dpi), figure size, and what ``interpolation='auto'`` must match.
ANTIALIASED = {
    "downsample": (5, 2),
    "exact": (5, 5),
    "double": (5, 10),
    "upsample_below_3": (3, 2.9),
    "upsample_above_3": (3, 9.1),
}


def _register_antialiased(name, img_size, fig_size):
    @classic(f"imshow_antialiased_{name}", figsize=(fig_size, fig_size))
    def _(fig):
        n = 100 * img_size
        ax = fig.subplots()
        ax.set_position([0, 0, 1, 1])
        ax.imshow(_uniform(19680801, n, n), interpolation="auto")


for _name, (_img_size, _fig_size) in ANTIALIASED.items():
    _register_antialiased(_name, _img_size, _fig_size)


@classic("imshow_zoom", figsize=(2.9, 2.9))
def _(fig):
    ax = fig.subplots()
    ax.imshow(_uniform(19680801, 300, 300), interpolation="auto")
    ax.set_xlim(10, 20)
    ax.set_ylim(10, 20)


@classic("image_alpha", remove_text=True)
def _(fig):
    Z = _uniform(0, 6, 6)
    ax1, ax2, ax3 = fig.subplots(1, 3)
    ax1.imshow(Z, alpha=1.0, interpolation="none")
    ax2.imshow(Z, alpha=0.5, interpolation="none")
    ax3.imshow(Z, alpha=0.5, interpolation="nearest")


@image_test("imshow_alpha")
def _(fig):
    rgbf = _uniform(19680801, 6, 6, 3).astype(np.float32)
    rgbu = np.uint8(rgbf * 255)
    ((ax0, ax1), (ax2, ax3)) = fig.subplots(2, 2)
    ax0.imshow(rgbf, alpha=0.5)
    ax1.imshow(rgbf, alpha=0.75)
    ax2.imshow(rgbu, alpha=127 / 255)
    ax3.imshow(rgbu, alpha=191 / 255)


@classic("image_array_alpha")
def _(fig):
    x = np.linspace(0, 1)
    xx, yy = np.meshgrid(x, x)
    zz = np.exp(- 3 * ((xx - 0.5) ** 2) + (yy - 0.7 ** 2))
    alpha = zz / zz.max()
    ax = fig.add_subplot()
    ax.imshow(zz, alpha=alpha, cmap="viridis", interpolation="nearest")


def _register_alpha_param(stage):
    @classic(f"interpolation_stage_rgba_respects_alpha_param_{stage}")
    def _(fig):
        axs = fig.subplots(2, 3)
        ny, nx = 3, 3
        draws = rng(19680801)
        array_alpha = draws.uniform(ny * nx).reshape(ny, nx)
        im_rgb = draws.uniform(ny * nx * 3).reshape(ny, nx, 3)
        im_rgba = np.concatenate((im_rgb, array_alpha.reshape((ny, nx, 1))), axis=-1)
        new_array_alpha = draws.uniform(ny * nx).reshape(ny, nx)
        axs[0][0].imshow(im_rgb)
        axs[0][1].imshow(im_rgb, interpolation_stage=stage, alpha=0.5)
        axs[0][2].imshow(im_rgb, interpolation_stage=stage, alpha=array_alpha)
        axs[1][0].imshow(im_rgba)
        axs[1][1].imshow(im_rgba, interpolation_stage=stage, alpha=0.5)
        axs[1][2].imshow(im_rgba, interpolation_stage=stage, alpha=new_array_alpha)


for _stage in ("data", "rgba"):
    _register_alpha_param(_stage)


# --- clipping, origin and placement ------------------------------------------


@image_test("image_clip")
def _(fig):
    ax = fig.subplots()
    im = ax.imshow([[1, 2], [3, 4]])
    im.set_clip_path(mpatches.Circle((0, 0), radius=1, transform=ax.transData))


@image_test("image_cliprect")
def _(fig):
    ax = fig.subplots()
    im = ax.imshow([[1, 2], [3, 4]], extent=(0, 5, 0, 5))
    im.set_clip_path(mpatches.Rectangle(xy=(1, 1), width=2, height=2,
                                        transform=im.axes.transData))


@classic("imshow_10_10_1")
def _(fig):
    arr = np.arange(100).reshape((10, 10, 1))
    ax = fig.subplots()
    ax.imshow(arr, interpolation="bilinear", extent=(1, 2, 1, 2))
    ax.set_xlim(0, 3)
    ax.set_ylim(0, 3)


@classic("no_interpolation_origin", remove_text=True)
def _(fig):
    axs = fig.subplots(2)
    axs[0].imshow(np.arange(100).reshape((2, 50)), origin="lower",
                  interpolation="none")
    axs[1].imshow(np.arange(100).reshape((2, 50)), interpolation="none")


@classic("image_shift", remove_text=True)
def _(fig):
    img_data = [[1 / x + 1 / y for x in range(1, 100)] for y in range(1, 100)]
    t_min = 734717.945208
    t_max = 734717.946366
    ax = fig.subplots()
    ax.imshow(img_data, norm=mcolors.LogNorm(), interpolation="none",
              extent=(t_min, t_max, 1, 100))
    ax.set_aspect("auto")


@image_test("image_composite_background", remove_text=True)
def _(fig):
    ax = fig.subplots()
    arr = np.arange(12).reshape(4, 3)
    ax.imshow(arr, extent=[0, 2, 15, 0])
    ax.imshow(arr, extent=[4, 6, 15, 0])
    ax.set_facecolor((1, 0, 0, 0.5))
    ax.set_xlim(0, 12)


@classic("image_composite_alpha", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ramp = np.concatenate((np.arange(0, 1.1, 0.1), np.arange(0, 1, 0.1)[::-1]))
    arr = np.zeros((11, 21, 4))
    arr[:, :, 0] = 1
    arr[:, :, 3] = ramp
    arr2 = np.zeros((21, 11, 4))
    arr2[:, :, 0] = 1
    arr2[:, :, 1] = 1
    arr2[:, :, 3] = ramp[:, np.newaxis]
    ax.imshow(arr, extent=[1, 2, 5, 0], alpha=0.3)
    ax.imshow(arr, extent=[2, 3, 5, 0], alpha=0.6)
    ax.imshow(arr, extent=[3, 4, 5, 0])
    ax.imshow(arr2, extent=[0, 5, 1, 2])
    ax.imshow(arr2, extent=[0, 5, 2, 3], alpha=0.6)
    ax.imshow(arr2, extent=[0, 5, 3, 4], alpha=0.3)
    ax.set_facecolor((0, 0.5, 0, 1))
    ax.set_xlim(0, 5)
    ax.set_ylim(5, 0)


@classic("clip_path_disables_compositing")
def _(fig):
    t = np.arange(9).reshape((3, 3))
    ax = fig.add_subplot()
    ax.imshow(t, clip_path=(mpath.Path([(0, 0), (0, 1), (1, 0)]), ax.transData))
    ax.imshow(t, clip_path=(mpath.Path([(1, 1), (1, 2), (2, 1)]), ax.transData))


@image_test("rasterize_dpi", figsize=(3, 1), dpi=10, remove_text=True)
def _(fig):
    img = np.asarray([[1, 2], [3, 4]])
    axs = fig.subplots(1, 3)
    axs[0].imshow(img)
    axs[1].plot([0, 1], [0, 1], linewidth=20., rasterized=True)
    axs[1].set(xlim=(0, 1), ylim=(-1, 2))
    axs[2].plot([0, 1], [0, 1], linewidth=20.)
    axs[2].set(xlim=(0, 1), ylim=(-1, 2))
    for ax in axs:
        ax.set_xticks([])
        ax.set_yticks([])
        ax.spines[:].set_visible(False)


@image_test("bbox_image_inverted", remove_text=True)
def _(fig):
    ax = fig.subplots()
    bbox_im = BboxImage(
        mtransforms.TransformedBbox(mtransforms.Bbox([[100, 100], [0, 0]]),
                                    ax.transData),
        interpolation="nearest")
    bbox_im.set_data(np.arange(100).reshape((10, 10)))
    bbox_im.set_clip_on(False)
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 100)
    ax.add_artist(bbox_im)
    bbox_im = BboxImage(
        mtransforms.TransformedBbox(mtransforms.Bbox([[0.1, 0.2], [0.3, 0.25]]),
                                    fig.transFigure),
        interpolation="nearest")
    bbox_im.set_data(np.identity(10))
    bbox_im.set_clip_on(False)
    ax.add_artist(bbox_im)


@image_test("zoom_and_clip_upper_origin", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.imshow(np.arange(100).reshape((10, 10)))
    ax.set_ylim(2.0, -0.5)
    ax.set_xlim(-0.5, 2.0)


@classic("log_scale_image", remove_text=True)
def _(fig):
    Z = np.zeros((10, 10))
    Z[::2] = 1
    ax = fig.subplots()
    ax.imshow(Z, extent=[1, 100, 1, 100], cmap="viridis", vmax=1, vmin=-1,
              aspect="auto")
    ax.set(yscale="log")


@classic("rotate_image", remove_text=True)
def _(fig):
    Z = _gaussians(0.25)
    ax1 = fig.subplots(1, 1)
    im1 = ax1.imshow(Z, interpolation="none", cmap="viridis", origin="lower",
                     extent=[-2, 4, -3, 2], clip_on=True)
    trans_data2 = mtransforms.Affine2D().rotate_deg(30) + ax1.transData
    im1.set_transform(trans_data2)
    x1, x2, y1, y2 = im1.get_extent()
    ax1.plot([x1, x2, x2, x1, x1], [y1, y1, y2, y2, y1], "r--", lw=3,
             transform=trans_data2)
    ax1.set_xlim(2, 5)
    ax1.set_ylim(0, 4)


@image_test("image_placement", remove_text=True)
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 0, 1, 1, 0], [0, 1, 1, 0, 0], color="r", lw=0.1)
    ax.imshow(rng(19680801).normal(256).reshape(16, 16), cmap="Blues",
              extent=(0, 1, 0, 1), interpolation="none", vmin=-1, vmax=1)
    ax.set_xlim(-0.1, 1 + 0.1)
    ax.set_ylim(-0.1, 1 + 0.1)


# --- masks, norms and extreme values -----------------------------------------


@classic("mask_image_over_under", remove_text=True)
def _(fig):
    Z = _gaussians(0.025, scale=10)
    palette = matplotlib.colormaps["gray"].with_extremes(over="r", under="g", bad="b")
    Zm = np.ma.masked_where(Z > 1.2, Z)
    ax1, ax2 = fig.subplots(1, 2)
    im = ax1.imshow(Zm, interpolation="bilinear", cmap=palette,
                    norm=mcolors.Normalize(vmin=-1.0, vmax=1.0, clip=False),
                    origin="lower", extent=[-3, 3, -3, 3])
    ax1.set_title("Green=low, Red=high, Blue=bad")
    fig.colorbar(im, extend="both", orientation="horizontal", ax=ax1, aspect=10)
    im = ax2.imshow(Zm, interpolation="nearest", cmap=palette,
                    norm=mcolors.BoundaryNorm([-1, -0.5, -0.2, 0, 0.2, 0.5, 1],
                                              ncolors=256, clip=False),
                    origin="lower", extent=[-3, 3, -3, 3])
    ax2.set_title("With BoundaryNorm")
    fig.colorbar(im, extend="both", spacing="proportional",
                 orientation="horizontal", ax=ax2, aspect=10)


@classic("mask_image", remove_text=True)
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    A = np.ones((5, 5))
    A[1:2, 1:2] = np.nan
    ax1.imshow(A, interpolation="nearest")
    A = np.zeros((5, 5), dtype=bool)
    A[1:2, 1:2] = True
    A = np.ma.masked_array(np.ones((5, 5), dtype=np.uint16), A)
    ax2.imshow(A, interpolation="nearest")


@classic("imshow_endianess", remove_text=True)
def _(fig):
    x = np.arange(10)
    X, Y = np.meshgrid(x, x)
    Z = np.hypot(X - 5, Y - 5)
    ax1, ax2 = fig.subplots(1, 2)
    kwargs = dict(origin="lower", interpolation="nearest", cmap="viridis")
    ax1.imshow(Z.astype("<f8"), **kwargs)
    ax2.imshow(Z.astype(">f8"), **kwargs)


#: ``sorted(mimage._interpd_)`` without ``auto`` and ``antialiased``.
INTERPOLATIONS = ["bessel", "bicubic", "bilinear", "blackman", "catrom",
                  "gaussian", "hamming", "hanning", "hermite", "kaiser",
                  "lanczos", "mitchell", "nearest", "none", "quadric", "sinc",
                  "spline16", "spline36"]


@image_test("imshow_masked_interpolation", remove_text=True)
def _(fig):
    cmap = matplotlib.colormaps["viridis"].with_extremes(over="r", under="b", bad="k")
    N = 20
    n = mcolors.Normalize(vmin=0, vmax=N * N - 1)
    data = np.arange(N * N, dtype=float).reshape(N, N)
    data[5, 5] = -1
    data[15, 5] = 1e5
    data[15, 15] = np.inf
    mask = np.zeros_like(data).astype("bool")
    mask[5, 15] = True
    data = np.ma.masked_array(data, mask)
    ax_grid = fig.subplots(3, 6)
    for interp, ax in zip(INTERPOLATIONS, ax_grid.ravel()):
        ax.set_title(interp)
        ax.imshow(data, norm=n, cmap=cmap, interpolation=interp)
        ax.axis("off")


@image_test("imshow_flatfield", remove_text=True)
def _(fig):
    ax = fig.subplots()
    im = ax.imshow(np.ones((5, 5)), interpolation="nearest")
    im.set_clim(.5, 1.5)


@image_test("imshow_bignumbers", remove_text=True,
            rc={"image.interpolation": "nearest"})
def _(fig):
    ax = fig.subplots()
    pc = ax.imshow(np.array([[1, 2, 1e12], [3, 1, 4]], dtype=np.uint64))
    pc.set_clim(0, 5)


@image_test("imshow_bignumbers_real", remove_text=True,
            rc={"image.interpolation": "nearest"})
def _(fig):
    ax = fig.subplots()
    pc = ax.imshow(np.array([[2., 1., 1.e22], [4., 1., 3.]]))
    pc.set_clim(0, 5)


@classic("norm_change")
def _(fig):
    data = np.full((5, 5), 1, dtype=np.float64)
    data[0:2, :] = -1
    masked_data = np.ma.array(data, mask=False)
    masked_data.mask[0:2, 0:2] = True
    cmap = matplotlib.colormaps["viridis"].with_extremes(under="w")
    ax = fig.subplots()
    im = ax.imshow(data, norm=mcolors.LogNorm(vmin=0.5, vmax=1),
                   extent=(0, 5, 0, 5), interpolation="nearest", cmap=cmap)
    im.set_norm(mcolors.Normalize(vmin=-2, vmax=2))
    im = ax.imshow(masked_data, norm=mcolors.LogNorm(vmin=0.5, vmax=1),
                   extent=(5, 10, 5, 10), interpolation="nearest", cmap=cmap)
    im.set_norm(mcolors.Normalize(vmin=-2, vmax=2))
    ax.set(xlim=(0, 10), ylim=(0, 10))


def _register_huge_range_log(name, x):
    @classic(f"huge_range_log_{name}")
    def _(fig):
        data = np.full((5, 5), x, dtype=np.float64)
        data[0:2, :] = 1E20
        ax = fig.subplots()
        ax.imshow(data, norm=mcolors.LogNorm(vmin=1, vmax=data.max()),
                  interpolation="nearest", cmap="viridis")


for _name, _x in (("negative", -1), ("positive", 1)):
    _register_huge_range_log(_name, _x)


@classic("str_norms")
def _(fig):
    t = _uniform(19680801, 10, 10) * .8 + .1
    axts = fig.subplots(1, 5)
    axts[0].imshow(t, norm="log")
    axts[1].imshow(t, norm="log", vmin=.2)
    axts[2].imshow(t, norm="symlog")
    axts[3].imshow(t, norm="symlog", vmin=.3, vmax=.7)
    axts[4].imshow(t, norm="logit", vmin=.3, vmax=.7)


@classic("spy_box")
def _(fig):
    ax_test = fig.subplots(1, 3)
    plot_data = ([[1, 1], [1, 1]], [[0, 0], [0, 0]], [[0, 1], [1, 0]])
    for ax, z, title in zip(ax_test, plot_data, ["ones", "zeros", "mixed"]):
        ax.set_title(title)
        ax.spy(z)


# --- non-uniform images --------------------------------------------------------


@image_test("nonuniform_and_pcolor", figsize=(3, 3))
def _(fig):
    axs = fig.subplots(3, sharex=True, sharey=True)
    for ax, interpolation in zip(axs, ["nearest", "bilinear"]):
        im = NonUniformImage(ax, interpolation=interpolation)
        im.set_data(np.arange(3) ** 2, np.arange(3) ** 2,
                    np.arange(9).reshape((3, 3)))
        ax.add_image(im)
    axs[2].pcolorfast(
        np.arange(4) ** 2, np.arange(4) ** 2, np.arange(9).reshape((3, 3)))
    for ax in axs:
        ax.set_axis_off()
        ax.set(xlim=(0, 10))


@image_test("nonuniform_logscale")
def _(fig):
    axs = fig.subplots(ncols=3, nrows=1)
    for i, ax in enumerate(axs):
        im = NonUniformImage(ax)
        im.set_data(np.arange(1, 4) ** 2, np.arange(1, 4) ** 2,
                    np.arange(9).reshape((3, 3)))
        ax.set_xlim(1, 16)
        ax.set_ylim(1, 16)
        ax.set_box_aspect(1)
        if i == 1:
            ax.set_xscale("log", base=2)
            ax.set_yscale("log", base=2)
        if i == 2:
            ax.set_xscale("log", base=4)
            ax.set_yscale("log", base=4)
        ax.add_image(im)


# --- antialiasing and interpolation stages --------------------------------------


@image_test("rgba_antialias", figsize=(3.5, 3.5), remove_text=True)
def _(fig):
    axs = fig.subplots(2, 2, sharex=False, sharey=False)
    fig.set_layout_engine("constrained")
    N = 250
    aa, a = _rings(N, f0=10, k=75)
    aa[:, int(N / 2):] = a[:, int(N / 2):]
    aa[20:50, 20:50] = np.nan
    aa[70:90, 70:90] = 1e6
    aa[70:90, 20:30] = -1e6
    aa[70:90, 195:215] = 1e6
    aa[20:30, 195:215] = -1e6
    cmap = matplotlib.colormaps["RdBu_r"].with_extremes(over="yellow", under="cyan")
    axs = axs.flatten()
    axs[0].imshow(aa, interpolation="nearest", cmap=cmap, vmin=-1.2, vmax=1.2)
    axs[0].set_xlim(N / 2 - 25, N / 2 + 25)
    axs[0].set_ylim(N / 2 + 50, N / 2 - 10)
    axs[1].imshow(aa, interpolation="nearest", cmap=cmap, vmin=-1.2, vmax=1.2)
    axs[2].imshow(aa, interpolation="auto", interpolation_stage="data",
                  cmap=cmap, vmin=-1.2, vmax=1.2)
    axs[3].imshow(aa, interpolation="auto", interpolation_stage="rgba",
                  cmap=cmap, vmin=-1.2, vmax=1.2)


@classic("upsample_interpolation_stage")
def _(fig):
    ax = fig.subplots()
    ax.imshow(_uniform(19680801 + 9, 4, 4), interpolation="bilinear",
              cmap="viridis", interpolation_stage="auto")


@classic("downsample_interpolation_stage")
def _(fig):
    ax = fig.subplots()
    ax.imshow(_uniform(19680801, 400, 400), interpolation="auto", cmap="viridis",
              interpolation_stage="auto")


@image_test("downsampling", figsize=(7, 6), remove_text=True)
def _(fig):
    N = 450
    aa, a = _rings(N, f0=5, k=100)
    aa[:, int(N / 3):] = a[:, int(N / 3):]
    a = aa
    axs = fig.subplots(2, 3)
    fig.set_layout_engine("compressed")
    axs[0, 0].imshow(a, interpolation="nearest", interpolation_stage="rgba",
                     cmap="RdBu_r")
    axs[0, 0].set_xlim(125, 175)
    axs[0, 0].set_ylim(250, 200)
    axs[0, 0].set_title("Zoom")
    for ax, interp, space in zip(axs.flat[1:],
                                 ["nearest", "nearest", "hanning", "hanning", "auto"],
                                 ["data", "rgba", "data", "rgba", "auto"]):
        ax.imshow(a, interpolation=interp, interpolation_stage=space, cmap="RdBu_r")
        ax.set_title(f"interpolation='{interp}'\nspace='{space}'")


@image_test("downsampling_speckle", figsize=(5, 2.7), remove_text=True)
def _(fig):
    axs = fig.subplots(1, 2, sharex=True, sharey=True)
    fig.set_layout_engine("compressed")
    img = ((np.arange(1024).reshape(-1, 1) * np.ones(720)) // 50).T
    cm = matplotlib.colormaps["viridis"].with_extremes(over="m")
    norm = mcolors.LogNorm(vmin=3, vmax=11)
    axs[0].set_title("interpolation='auto', stage='rgba'")
    axs[0].imshow(np.triu(img), cmap=cm, norm=norm, interpolation_stage="rgba")
    axs[1].set_title("interpolation='auto', stage='auto'")
    axs[1].imshow(np.triu(img), cmap=cm, norm=norm)


@image_test("upsampling", figsize=(6.5, 3), remove_text=True)
def _(fig):
    a = _uniform(19680801 + 9, 4, 4)
    axs = fig.subplots(1, 3)
    fig.set_layout_engine("compressed")
    im = axs[0].imshow(a, cmap="viridis")
    axs[0].set_title(
        "interpolation='auto'\nstage='antialaised'\n(default for upsampling)")
    axs[1].imshow(a, cmap="viridis", interpolation="sinc")
    axs[1].set_title(
        "interpolation='sinc'\nstage='auto'\n(default for upsampling)")
    axs[2].imshow(a, cmap="viridis", interpolation="sinc", interpolation_stage="rgba")
    axs[2].set_title("interpolation='sinc'\nstage='rgba'")
    fig.colorbar(im, ax=axs, shrink=0.7, extend="both")


# --- pixel alignment and transformed images -------------------------------------


@classic("nn_pixel_alignment")
def _(fig):
    axs = fig.subplots(2, 3)
    for j, N in enumerate([3, 7, 11]):
        data = np.arange(N**2).reshape((N, N)) % 4
        seps = np.arange(-0.5, N)
        for i in range(2):
            transform = axs[i, j].transData
            if i == 1:
                transform = NonAffineIdentity() + transform
            axs[i, j].imshow(data, cmap="Grays", interpolation="nearest",
                             transform=transform)
            axs[i, j].set_axis_off()
            axs[i, j].vlines(seps, -1, N, lw=0.5, color="red", ls="dashed")
            axs[i, j].hlines(seps, -1, N, lw=0.5, color="red", ls="dashed")


@image_test("alignment_half_display_pixels", figsize=(5, 2))
def _(fig):
    fig.set_facecolor("g")
    corner_x = [0.01, 0.199, 0.41, 0.599, 0.809]
    corner_y = [0.05, 0.53]
    axs = []
    for cy in corner_y:
        for ix, cx in enumerate(corner_x):
            my = cy + 0.0125 if ix in [1, 3, 4] else cy
            axs.append(fig.add_axes([cx, my, 0.17, 0.425], xticks=[], yticks=[]))
    N = 10
    data = np.arange(N**2).reshape((N, N)) % 9
    seps = np.arange(-0.5, N)
    for i, ax in enumerate(axs):
        ax.set_facecolor("m")
        transform = (NonAffineIdentity() + ax.transData if i % 4 >= 2
                     else ax.transData)
        ax.imshow(data, cmap="Blues",
                  interpolation="hanning" if i % 5 == 4 else "nearest",
                  origin="upper" if i >= 5 else "lower",
                  transform=transform)
        ax.vlines(seps, -0.5, N - 0.5, lw=0.5, color="red", ls=(0, (2, 4)))
        ax.hlines(seps, -0.5, N - 0.5, lw=0.5, color="red", ls=(0, (2, 4)))
        for spine in ax.spines:
            ax.spines[spine].set_linestyle((0, (5, 10)))


@classic("image_bounds_handling")
def _(fig):
    axs = fig.subplots(2, 3)
    N = 11
    for j, interpolation in enumerate(["nearest", "hanning", "bilinear"]):
        data = np.arange(N**2).reshape((N, N))
        data = data / N**2 + (data % 4) / 6
        rotation = mtransforms.Affine2D().rotate_around(N / 2 - 0.5, N / 2 - 0.5, 1)
        for i in range(2):
            transform = rotation + axs[i, j].transData
            if i == 1:
                transform = NonAffineIdentity() + transform
            axs[i, j].imshow(data, cmap="Grays", interpolation=interpolation,
                             transform=transform)
            axs[i, j].set_axis_off()
            box = mpatches.Rectangle((-0.5, -0.5), N, N, edgecolor="red",
                                     facecolor="none", lw=0.5, ls="dashed",
                                     transform=rotation + axs[i, j].transData)
            axs[i, j].add_artist(box)


@classic("rgba_clean_edges")
def _(fig):
    data = _uniform(19680801 + 9, 8, 8)
    data = np.stack([data, data])
    data[1, 2:4, 2:4] = np.nan
    rotation = mtransforms.Affine2D().rotate_around(3.5, 3.5, 1)
    axs = fig.subplots(1, 2)
    for i in range(2):
        axs[i].add_patch(mpatches.Rectangle((3.75, 2), 5, 5, color="black", zorder=0))
        axs[i].add_patch(mpatches.Rectangle((0, -2), 3.75, 4, color="gray", zorder=0))
        axs[i].add_patch(mpatches.Rectangle((3.75, -2), 5, 4, fc="black", ec="none",
                                            alpha=0.5, zorder=0))
        axs[i].imshow(data[i, ...], interpolation="bilinear",
                      interpolation_stage="rgba",
                      transform=rotation + axs[i].transData)
        axs[i].set_axis_off()
        axs[i].set_xlim(-2.5, 9.5)
        axs[i].set_ylim(-2.5, 9.5)


@classic("affine_fill_to_edges")
def _(fig):
    axs = fig.subplots(2, 3)
    N = 7
    data = np.arange(N**2).reshape((N, N)) % 3
    transform = [mtransforms.Affine2D(),
                 mtransforms.Affine2D().translate(0, -N + 1).scale(1, -1),
                 mtransforms.Affine2D().translate(-N + 1, 0).scale(-1, 1)]
    for j in range(3):
        for i in range(2):
            origin = "upper" if i == 0 else "lower"
            axs[i, j].imshow(data, cmap="Grays", interpolation="hanning",
                             origin=origin,
                             transform=transform[j] + axs[i, j].transData)
            axs[i, j].set_axis_off()
            axs[i, j].vlines([-0.5, N - 0.5], -1, 2, lw=0.5, color="red")
            axs[i, j].vlines([-0.5, N - 0.5], N - 3, N, lw=0.5, color="red")
            axs[i, j].hlines([-0.5, N - 0.5], -1, 2, lw=0.5, color="red")
            axs[i, j].hlines([-0.5, N - 0.5], N - 3, N, lw=0.5, color="red")
