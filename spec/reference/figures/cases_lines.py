"""Line plots: ``Axes.plot`` styling plus step/stairs, reference lines and spans."""

import numpy as np

from spec_support import case, rng

X = np.linspace(0, 2 * np.pi, 50)


# --- plot basics -----------------------------------------------------------


@case("lines/y_only")
def _(fig):
    ax = fig.subplots()
    ax.plot(np.sin(X))


@case("lines/xy_two_series")
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))
    ax.plot(X, np.cos(X))


@case("lines/color_cycle_wraps", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    for i in range(12):
        ax.plot([0, 1], [i, i + 0.5], linewidth=3)


@case("lines/columns_of_2d_array", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 1, 20)
    ax.plot(t, np.column_stack([t, t**2, t**3]))


@case("lines/multiple_pairs_one_call", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 2, 30)
    ax.plot(t, t, "r--", t, t**2, "bs", t, t**1.5, "g^")


@case("lines/fmt_strings", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(10)
    for i, fmt in enumerate(["ro--", "g^", "b-.", "k:", "cs-", "m+", "yD:", "C7x-"]):
        ax.plot(t, t + 3 * i, fmt)



# --- line styles -----------------------------------------------------------


@case("lines/linestyles_named", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, ls in enumerate(["-", "--", "-.", ":", "solid", "dashed", "dashdot", "dotted"]):
        ax.plot([0, 1], [i, i], linestyle=ls)


@case("lines/linestyles_dash_tuples", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    styles = [(0, (1, 1)), (0, (5, 2)), (5, (10, 3)), (0, (3, 5, 1, 5, 1, 5)), (0, (8, 2, 1, 2))]
    for i, ls in enumerate(styles):
        ax.plot([0, 1], [i, i], linestyle=ls, color="k")


@case("lines/set_dashes_and_gapcolor", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 0], dashes=[6, 2])
    ax.plot([0, 1], [1, 1], linestyle="--", gapcolor="tab:red", linewidth=3)
    ax.plot([0, 1], [2, 2], dashes=[2, 2, 10, 2], gapcolor="0.8", linewidth=3)


@case("lines/linewidths", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, lw in enumerate([0.25, 0.5, 1, 2, 4, 8]):
        ax.plot([0, 1], [i, i + 1], linewidth=lw)


@case("lines/dashes_scale_with_linewidth", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, lw in enumerate([0.5, 1, 2, 4]):
        ax.plot([0, 1], [i, i], "--", linewidth=lw, color="k")
        ax.plot([0, 1], [i + 0.4, i + 0.4], ":", linewidth=lw, color="b")


@case("lines/solid_capstyles", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, cap in enumerate(["butt", "round", "projecting"]):
        ax.plot([0.2, 0.8], [i, i], linewidth=14, solid_capstyle=cap)
    ax.set_xlim(0, 1)
    ax.set_ylim(-1, 3)


@case("lines/dash_capstyles", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, cap in enumerate(["butt", "round", "projecting"]):
        ax.plot([0, 1], [i, i], "--", linewidth=8, dash_capstyle=cap)
    ax.set_ylim(-1, 3)


@case("lines/joinstyles", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    zig = np.array([0.0, 1.0, 0.0])
    for i, join in enumerate(["miter", "round", "bevel"]):
        ax.plot([3 * i, 3 * i + 1, 3 * i + 2], zig + 0.5, linewidth=12, solid_joinstyle=join)
    ax.set_xlim(-1, 9)
    ax.set_ylim(0, 2)


# --- markers ---------------------------------------------------------------


@case("lines/markers_filled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    for i, m in enumerate(["o", "v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X"]):
        ax.plot([0, 1, 2], [i, i, i], marker=m, markersize=10, linestyle="")


@case("lines/markers_unfilled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    markers = [".", ",", "1", "2", "3", "4", "+", "x", "|", "_", 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
    for i, m in enumerate(markers):
        ax.plot([i % 11], [i // 11], marker=m, markersize=12, markeredgewidth=1.5, color="k")
    ax.set_xlim(-1, 11)
    ax.set_ylim(-1, 2)


@case("lines/markers_polygon_tuples_and_vertices", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    markers = [(5, 0), (5, 1), (5, 2), (6, 0, 30), (3, 1, 90), [(-1, -1), (1, -1), (0, 1.5)]]
    for i, m in enumerate(markers):
        ax.plot([i], [0], marker=m, markersize=18, linestyle="")
    ax.set_xlim(-1, 6)


@case("lines/markers_mathtext", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    for i, m in enumerate([r"$\clubsuit$", r"$f$", r"$\alpha$", r"$\heartsuit$"]):
        ax.plot([0, 1, 2], [i, i + 0.5, i], marker=m, markersize=15)


@case("lines/marker_size_edge_face", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = np.arange(6)
    ax.plot(t, t, "o-", markersize=4)
    ax.plot(t, t + 2, "o-", markersize=12, markerfacecolor="yellow", markeredgecolor="red", markeredgewidth=2)
    ax.plot(t, t + 4, "s--", markersize=9, markerfacecolor="none", markeredgecolor="k")
    ax.plot(t, t + 6, "D", markersize=8, markeredgewidth=0)


@case("lines/fillstyles", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    for i, fs in enumerate(["full", "left", "right", "bottom", "top", "none"]):
        ax.plot([i], [0], "o", markersize=20, fillstyle=fs, markerfacecoloralt="tab:red", color="tab:blue")
        ax.plot([i], [1], "D", markersize=20, fillstyle=fs, markerfacecoloralt="0.8", color="k")
    ax.set_xlim(-1, 6)
    ax.set_ylim(-1, 2)


@case("lines/markevery_variants", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 10, 101)
    for i, every in enumerate([5, (3, 10), [0, 25, 50, 75, 100], slice(10, 90, 20), 0.2]):
        ax.plot(t, np.sin(t) + 2.5 * i, "o-", markevery=every, markersize=5)


# --- drawing styles, alpha, antialiasing ----------------------------------


@case("lines/drawstyles", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(8)
    y = np.array([1, 3, 2, 4, 3, 5, 4, 6], dtype=float)
    for i, ds in enumerate(["default", "steps-pre", "steps-mid", "steps-post"]):
        ax.plot(t, y + 6 * i, "o-", drawstyle=ds)


@case("lines/alpha_overlap", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 1, 2)
    for i in range(5):
        ax.plot(t, t + 0.1 * i, linewidth=20, alpha=0.3, color="tab:blue")
    ax.plot(t, 1 - t, linewidth=20, color="tab:red", alpha=0.6)


@case("lines/antialiased_off", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X), antialiased=False, linewidth=2)
    ax.plot(X, np.cos(X), "o", antialiased=False)


@case("lines/zorder_overrides_call_order", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1], linewidth=15, zorder=3)
    ax.plot([0, 1], [1, 0], linewidth=15, zorder=1)
    ax.plot([0, 1], [0.5, 0.5], linewidth=15, zorder=2)


@case("lines/clip_on_false_markers", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2], [0, 1, 0], "o-", markersize=20, clip_on=False)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1)


# --- data edge cases -------------------------------------------------------


@case("lines/nan_and_inf_gaps", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(20, dtype=float)
    y = np.sin(t / 3)
    y[[4, 10, 11, 17]] = np.nan
    y[7] = np.inf
    ax.plot(t, y, "o-")
    ax.plot(t, y + 2, drawstyle="steps-mid")


@case("lines/long_noisy_line_simplified")
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 10, 50_000)
    ax.plot(t, np.sin(t) + rng(1).normal(t.size, scale=0.1), linewidth=0.5)


@case("lines/loglog", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.logspace(-2, 3, 60)
    ax.loglog(t, t**2, "o-", markersize=3)
    ax.loglog(t, 10 * np.sqrt(t))


@case("lines/semilogy_exponential", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.linspace(0, 10, 40)
    ax.semilogy(t, np.exp(t))
    ax.semilogy(t, np.exp(t / 2), "--")


@case("lines/semilogx_base2", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = 2.0 ** np.arange(0, 12)
    ax.semilogx(t, np.log2(t), "s-", base=2)


@case("lines/single_point_and_constant", figsize=(6, 3))
def _(fig):
    ax = fig.subplots(1, 2)
    ax[0].plot([3], [5], "o")
    ax[1].plot([0, 1, 2, 3], [2, 2, 2, 2])


# --- step, stairs ----------------------------------------------------------


@case("lines/step_where", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(10)
    y = np.sin(t / 2)
    ax.step(t, y + 4, where="pre")
    ax.step(t, y + 2, where="mid")
    ax.step(t, y, where="post")
    ax.plot(t, y, "o", color="0.6")


@case("lines/stairs_default_and_fill", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    values = np.array([1, 3, 2, 5, 4, 2], dtype=float)
    ax.stairs(values, linewidth=2)
    ax.stairs(values + 6, np.array([0, 1, 1.5, 3, 4, 5.5, 6]), fill=True, alpha=0.5)


@case("lines/stairs_baseline_and_horizontal", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    values = np.array([3, 1, 4, 1, 5], dtype=float)
    ax1.stairs(values, baseline=2, fill=True, hatch="//")
    ax1.stairs(values, baseline=None, color="k")
    ax2.stairs(values, orientation="horizontal", fill=True)


# --- vlines, hlines, axhline, axvline, axline, spans --------------------------


@case("lines/vlines_hlines", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = np.arange(1, 8)
    ax.vlines(t, 0, np.sqrt(t), colors="C0", linewidth=3)
    ax.hlines([0.5, 1.5, 2.5], 1, [3, 5, 7], colors=["r", "g", "b"], linestyles=["solid", "dashed", "dotted"])


@case("lines/axhline_axvline_fractions", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 10], [0, 5])
    ax.axhline(2.5, color="k")
    ax.axhline(4, xmin=0.25, xmax=0.75, color="r", linestyle="--", linewidth=2)
    ax.axvline(3, color="g", linewidth=3)
    ax.axvline(7, ymin=0.1, ymax=0.4, color="m")


@case("lines/axline_points_and_slope", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 4], [0, 3], "o")
    ax.axline((0, 0), (1, 1), color="C1")
    ax.axline((0, 1), slope=-0.5, color="C2", linestyle="--")
    ax.axline((2, 0), (2, 1), color="C3")


@case("lines/axhspan_axvspan", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(X, np.sin(X))
    ax.axhspan(-0.25, 0.25, color="yellow", alpha=0.5)
    ax.axvspan(1, 2, color="tab:green", alpha=0.3)
    ax.axvspan(4, 5, ymin=0.2, ymax=0.6, facecolor="none", edgecolor="r", hatch="x")
