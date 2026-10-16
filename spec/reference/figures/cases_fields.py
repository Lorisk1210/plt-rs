"""Vector fields: quiver (+ quiverkey), barbs and streamplot."""

import numpy as np

from spec_support import case


def _grid(n=9, extent=2.0):
    x = np.linspace(-extent, extent, n)
    return np.meshgrid(x, x)


def _swirl(xx, yy):
    """Rotational field with a sink: u, v."""
    return -yy - 0.3 * xx, xx - 0.3 * yy


def _stream_field(n=40):
    x = np.linspace(-3.0, 3.0, n)
    y = np.linspace(-3.0, 3.0, n)
    xx, yy = np.meshgrid(x, y)
    u = -1 - xx**2 + yy
    v = 1 + xx - yy**2
    return x, y, u, v


# --- quiver

@case("fields/quiver_default")
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid()
    ax.quiver(xx, yy, *_swirl(xx, yy))


@case("fields/quiver_uv_only", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(6)
    ax.quiver(*_swirl(xx, yy))


@case("fields/quiver_xy_scale_units", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.quiver([0, 0, 1], [0, 1, 1], [1, 2, -1], [1, -0.5, 1], angles="xy", scale_units="xy", scale=1)
    ax.set_xlim(-1, 3)
    ax.set_ylim(-1, 3)


@case("fields/quiver_scale_width", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(7)
    ax.quiver(xx, yy, *_swirl(xx, yy), scale=40, width=0.012, color="tab:red")


@case("fields/quiver_units_xy", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(5)
    ax.quiver(xx, yy, *_swirl(xx, yy), units="xy", width=0.05, scale=3)
    ax.set_aspect("equal")


@case("fields/quiver_units_dots_inches", figsize=(5, 2.5))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    xx, yy = _grid(5)
    ax1.quiver(xx, yy, *_swirl(xx, yy), units="dots", width=2)
    ax2.quiver(xx, yy, *_swirl(xx, yy), units="inches", width=0.03, scale_units="inches", scale=4)


@case("fields/quiver_pivot", figsize=(6, 2.5))
def _(fig):
    axs = fig.subplots(1, 3)
    xx, yy = _grid(5)
    for ax, pivot in zip(axs, ["tail", "middle", "tip"]):
        ax.quiver(xx, yy, *_swirl(xx, yy), pivot=pivot, scale=12)
        ax.plot(xx.ravel(), yy.ravel(), "r.", ms=3)
        ax.set_title(pivot)


@case("fields/quiver_head_shape", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(5)
    ax.quiver(xx, yy, *_swirl(xx, yy), headwidth=6, headlength=8, headaxislength=7, width=0.008)


@case("fields/quiver_flat_head_edges", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(5)
    ax.quiver(xx, yy, *_swirl(xx, yy), headaxislength=0, headwidth=4, color="gold",
              edgecolor="k", linewidth=0.8)


@case("fields/quiver_color_by_c", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(8)
    u, v = _swirl(xx, yy)
    ax.quiver(xx, yy, u, v, np.hypot(u, v), cmap="viridis")


@case("fields/quiver_small_vectors", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(6)
    u, v = _swirl(xx, yy)
    scale = np.hypot(xx, yy) ** 2
    ax.quiver(xx, yy, u * scale, v * scale, minshaft=2, minlength=0.5)


@case("fields/quiverkey", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy = _grid(7)
    q = ax.quiver(xx, yy, *_swirl(xx, yy))
    ax.quiverkey(q, 0.85, 1.03, 2, r"$2\ \mathrm{m/s}$", labelpos="E", coordinates="axes")
    ax.quiverkey(q, 0.1, 0.1, 1, "1 unit", labelpos="N", coordinates="data", color="r")


# --- barbs

def _barb_data():
    x = np.arange(6)
    xx, yy = np.meshgrid(x, x[:4])
    speed = np.array([0, 3, 5, 10, 15, 25, 35, 50, 65, 80, 95, 110,
                      120, 140, 7, 12, 22, 47, 52, 57, 8, 18, 28, 38], dtype=float)
    angle = np.linspace(0.0, 2 * np.pi, 24, endpoint=False)
    return xx, yy, (speed * np.cos(angle)).reshape(4, 6), (speed * np.sin(angle)).reshape(4, 6)


@case("fields/barbs_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barbs(*_barb_data())
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 4)


@case("fields/barbs_length_sizes_fill", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barbs(*_barb_data(), length=8, fill_empty=True,
             sizes=dict(emptybarb=0.3, spacing=0.2, height=0.4, width=0.3))
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 4)


@case("fields/barbs_flip_colors", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barbs(*_barb_data(), flip_barb=True, barbcolor="tab:red", flagcolor="tab:blue", pivot="middle")
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 4)


@case("fields/barbs_increments_no_rounding", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.barbs(*_barb_data(), barb_increments=dict(half=10, full=20, flag=100), rounding=False)
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 4)


@case("fields/barbs_colormapped", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    xx, yy, u, v = _barb_data()
    ax.barbs(xx, yy, u, v, np.hypot(u, v), cmap="plasma")
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 4)


# --- streamplot

@case("fields/streamplot_default")
def _(fig):
    ax = fig.subplots()
    ax.streamplot(*_stream_field())


@case("fields/streamplot_density_color", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    x, y, u, v = _stream_field()
    ax.streamplot(x, y, u, v, density=[0.5, 1.5], color=np.hypot(u, v), cmap="autumn")


@case("fields/streamplot_linewidth_array", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    x, y, u, v = _stream_field()
    speed = np.hypot(u, v)
    ax.streamplot(x, y, u, v, density=0.6, color="k", linewidth=5 * speed / speed.max())


@case("fields/streamplot_start_points", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    starts = np.array([[-2.0, -2.0], [-1.0, 0.0], [0.0, 2.0], [1.5, -1.5], [2.5, 2.5]])
    ax.streamplot(*_stream_field(), start_points=starts, color="tab:blue", linewidth=2)
    ax.plot(starts[:, 0], starts[:, 1], "ko")


@case("fields/streamplot_unbroken", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.streamplot(*_stream_field(), density=0.6, broken_streamlines=False)


@case("fields/streamplot_forward_only", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.streamplot(*_stream_field(), density=0.8, integration_direction="forward")


@case("fields/streamplot_arrowstyle", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    ax.streamplot(*_stream_field(), density=0.7, arrowstyle="->", arrowsize=2, num_arrows=3)


@case("fields/streamplot_masked", figsize=(4, 4))
def _(fig):
    ax = fig.subplots()
    x, y, u, v = _stream_field()
    mask = np.zeros(u.shape, dtype=bool)
    mask[15:25, 15:25] = True
    u = np.ma.array(u, mask=mask)
    ax.streamplot(x, y, u, v, color="r")
    ax.imshow(~mask, extent=(-3, 3, -3, 3), alpha=0.5, cmap="gray", aspect="auto", origin="lower")
