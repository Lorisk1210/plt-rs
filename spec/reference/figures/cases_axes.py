"""Axes decorations and coordinate systems: titles, labels, limits, aspect,
grids, ticks, spines, twins, secondary axes, scales and tick formatting."""

import numpy as np

from spec_support import case

T = np.linspace(0, 10, 50)


def _line(ax):
    ax.plot(T, np.sin(T))


# --- titles and labels -----------------------------------------------------


@case("axes/title_locations")
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_title("Left title", loc="left")
    ax.set_title("Center title")
    ax.set_title("Right title", loc="right")


@case("axes/title_pad_y_fontdict", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_title("Padded bold title", pad=20, fontdict={"fontsize": 16, "fontweight": "bold", "color": "navy"})
    ax.set_title("low", loc="left", y=0.9, fontsize="small")


@case("axes/suptitle_supxlabel_supylabel", figsize=(6, 4))
def _(fig):
    axs = fig.subplots(1, 2)
    for ax in axs:
        _line(ax)
    fig.suptitle("Figure title")
    fig.supxlabel("shared x label")
    fig.supylabel("shared y label")



@case("axes/xlabel_ylabel_default")
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xlabel("time [s]")
    ax.set_ylabel("amplitude")


@case("axes/xlabel_ylabel_loc_and_pad")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(left=0.18)
    _line(ax)
    ax.set_xlabel("x at right", loc="right")
    ax.set_ylabel("y at top", loc="top", labelpad=12, color="tab:red", fontsize=14)


@case("axes/mathtext_labels")
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_title(r"$\alpha^2 + \beta_i = \frac{1}{2}\sqrt{x}$")
    ax.set_xlabel(r"$\omega t$ [rad]")
    ax.set_ylabel(r"$\sin(\omega t)$")


# --- limits, margins and autoscaling ---------------------------------------


@case("axes/set_limits", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xlim(2, 8)
    ax.set_ylim(-2, 1.5)


@case("axes/reversed_limits", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xlim(10, 0)
    ax.set_ylim(1.5, -1.5)


@case("axes/invert_axes", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.invert_xaxis()
    _line(ax2)
    ax2.invert_yaxis()



@case("axes/margins", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.margins(0.25)
    _line(ax2)
    ax2.margins(x=0, y=-0.25)


@case("axes/autoscale_tight", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.autoscale(tight=True)


@case("axes/relim", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    line, = ax1.plot([0, 1], [0, 1])
    ax1.bar([2], [1])
    ax1.scatter([3.5], [-1.5])
    line.set_data([0, 4], [-2, 3])
    ax1.relim()
    ax1.autoscale_view()
    ax2.plot([0, 1], [0, 1])
    hidden, = ax2.plot([-5, 5], [-5, 5])
    hidden.set_visible(False)
    ax2.relim(visible_only=True)
    ax2.autoscale_view()



# --- aspect ----------------------------------------------------------------

CIRCLE = np.linspace(0, 2 * np.pi, 100)


def _ellipse(ax):
    ax.plot(3 * np.cos(CIRCLE), np.sin(CIRCLE))


@case("axes/aspect_equal_box", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _ellipse(ax)
    ax.set_aspect("equal")


@case("axes/aspect_equal_datalim", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _ellipse(ax)
    ax.set_aspect("equal", adjustable="datalim")


@case("axes/aspect_numeric_anchor", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _ellipse(ax)
    ax.set_aspect(2, anchor="SW")


@case("axes/box_aspect", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.set_box_aspect(1)
    _line(ax2)
    ax2.set_box_aspect(0.5)






@case("axes/axis_modes_equal_scaled_tight_square", figsize=(7, 6))
def _(fig):
    axs = fig.subplots(2, 2)
    fig.subplots_adjust(hspace=0.4)
    for ax, mode in zip(axs.flat, ["equal", "scaled", "tight", "square"]):
        _ellipse(ax)
        ax.axis(mode)
        ax.set_title(mode)


@case("axes/axis_off", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _ellipse(ax)
    ax.set_title("no axis decorations")
    ax.axis("off")


@case("axes/frame_off_and_facecolor", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.set_frame_on(False)
    _line(ax2)
    ax2.set_facecolor("lightyellow")


# --- grids -----------------------------------------------------------------


@case("axes/grid_default", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.grid(True)


@case("axes/grid_major_minor_styles", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.minorticks_on()
    ax.grid(which="major", color="0.4", linewidth=1)
    ax.grid(which="minor", color="0.8", linestyle=":", linewidth=0.8)


@case("axes/grid_y_only_styled", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.grid(axis="y", color="tab:red", linestyle="--", linewidth=0.5, alpha=0.7)


@case("axes/grid_below_bars", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(np.arange(5), [3, 5, 2, 4, 6])
    ax.set_axisbelow(True)
    ax.grid(True, linewidth=2)


# --- ticks -----------------------------------------------------------------


@case("axes/minorticks_on", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.minorticks_on()


@case("axes/tick_direction_in_inout", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    for ax, direction in [(ax1, "in"), (ax2, "inout")]:
        _line(ax)
        ax.minorticks_on()
        ax.tick_params(which="both", direction=direction)


@case("axes/tick_length_width_colors", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.tick_params(axis="x", length=10, width=2, colors="tab:red")
    ax.tick_params(axis="y", length=2, width=0.5, color="tab:blue", labelcolor="tab:green")


@case("axes/tick_labelsize_rotation_pad", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.tick_params(axis="x", labelsize=14, labelrotation=45, pad=8)
    ax.tick_params(axis="y", labelsize="x-small")


@case("axes/tick_labels_top_right")
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.tick_params(top=True, labeltop=True, right=True, labelright=True)


@case("axes/ticklabels_not_antialiased", figsize=(4, 3))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xticks([0, 2.5, 5, 7.5, 10])
    for label in ax.get_xticklabels():
        label.set_antialiased(False)


@case("axes/tick_labelbottom_false", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.tick_params(labelbottom=False, bottom=False, labelleft=False)


@case("axes/set_ticks_labels_minor", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xticks([0, np.pi, 2 * np.pi, 3 * np.pi], ["0", r"$\pi$", r"$2\pi$", r"$3\pi$"])
    ax.set_xticks(np.arange(0, 10, 0.5), minor=True)
    ax.set_yticks([-1, 0, 1])


@case("axes/ticklabel_rotation_alignment", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    names = ["January", "February", "March", "April", "May"]
    ax.plot(np.arange(5), [3, 1, 4, 1, 5])
    ax.set_xticks(np.arange(5), names, rotation=45, ha="right", rotation_mode="anchor")
    fig.subplots_adjust(bottom=0.25)
    ax.set_yticks([1, 3, 5], ["low", "mid", "high"], rotation=90, va="center")



@case("axes/locator_params_nbins", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.locator_params(axis="x", nbins=3)
    ax1.locator_params(axis="y", nbins=10)
    _line(ax2)
    ax2.locator_params(nbins=4, steps=[1, 2.5, 5])


@case("axes/negative_tick_labels_unicode_minus", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([-5, 5], [-0.003, 0.002])


# --- spines ----------------------------------------------------------------


@case("axes/spines_hidden_top_right", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.spines[["top", "right"]].set_visible(False)


@case("axes/spines_center_and_zero", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    t = np.linspace(-3, 5, 50)
    for ax, where in [(ax1, "center"), (ax2, "zero")]:
        ax.plot(t, np.sin(t) + 0.5)
        ax.spines[["left", "bottom"]].set_position(where)
        ax.spines[["top", "right"]].set_visible(False)


@case("axes/spines_outward_and_data", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    _line(ax1)
    ax1.spines[["left", "bottom"]].set_position(("outward", 10))
    ax1.spines[["top", "right"]].set_visible(False)
    _line(ax2)
    ax2.spines["left"].set_position(("data", 4))
    ax2.spines["bottom"].set_position(("axes", 0.3))


@case("axes/spine_bounds_and_style", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.spines["left"].set_bounds(-1, 1)
    ax.spines["bottom"].set_bounds(0, 10)
    ax.spines[["top", "right"]].set_visible(False)
    ax.spines["left"].set_linewidth(2)
    ax.spines["bottom"].set_color("tab:red")


# --- twins and secondary axes ----------------------------------------------


@case("axes/twinx")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(right=0.85)
    ax.plot(T, np.exp(T / 3), color="tab:blue")
    ax.set_ylabel("exp", color="tab:blue")
    ax2 = ax.twinx()
    ax2.plot(T, np.sin(T), color="tab:red")
    ax2.set_ylabel("sin", color="tab:red")


@case("axes/twiny", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    _line(ax)
    ax.set_xlabel("bottom")
    ax2 = ax.twiny()
    ax2.plot(T * 100, np.cos(T), "C1")
    ax2.set_xlabel("top")


@case("axes/twinx_in_front_zorder", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.bar(np.arange(5), [3, 4, 2, 5, 1], color="0.8")
    ax2 = ax.twinx()
    ax2.plot(np.arange(5), [10, 30, 20, 40, 25], "o-", linewidth=3)
    ax.set_zorder(ax2.get_zorder() + 1)
    ax.patch.set_visible(False)


@case("axes/secondary_xaxis_functions", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    deg = np.linspace(0, 360, 50)
    ax.plot(deg, np.sin(np.deg2rad(deg)))
    ax.set_xlabel("degrees")
    sec = ax.secondary_xaxis("top", functions=(np.deg2rad, np.rad2deg))
    sec.set_xlabel("radians")


@case("axes/secondary_yaxis_location")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(right=0.85)
    celsius = np.linspace(-10, 40, 20)
    ax.plot(np.arange(20), celsius)
    ax.set_ylabel("Celsius")
    sec = ax.secondary_yaxis("right", functions=(lambda c: c * 1.8 + 32, lambda f: (f - 32) / 1.8))
    sec.set_ylabel("Fahrenheit")


# --- shared axes -----------------------------------------------------------


@case("axes/sharex_sharey_label_outer")
def _(fig):
    axs = fig.subplots(2, 2, sharex=True, sharey=True)
    for i, ax in enumerate(axs.flat):
        ax.plot(T, np.sin(T + i) * (i + 1))
        ax.set_xlabel("x")
        ax.set_ylabel("y")
        ax.label_outer()


@case("axes/sharex_after_creation", figsize=(5, 4))
def _(fig):
    ax1 = fig.add_subplot(211)
    ax2 = fig.add_subplot(212, sharex=ax1)
    ax1.plot(T, np.sin(T))
    ax2.plot(T * 0.5, np.cos(T))


# --- scales ----------------------------------------------------------------


@case("axes/xscale_log_minor", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.logspace(-1, 3, 50)
    ax.plot(x, np.log10(x))
    ax.set_xscale("log")


@case("axes/yscale_log_base2_small_range", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3], [3, 5, 7])
    ax.set_yscale("log", base=2)


@case("axes/yscale_log_narrow_range", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([1, 2, 3], [20, 50, 70])
    ax.set_yscale("log")


@case("axes/yscale_symlog", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-50, 50, 101)
    ax.plot(x, x**3)
    ax.set_yscale("symlog", linthresh=10)


@case("axes/xscale_symlog_linscale", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-1000, 1000, 201)
    ax.plot(x, np.sign(x) * np.abs(x) ** 0.5)
    ax.set_xscale("symlog", linthresh=1, linscale=0.5)


@case("axes/yscale_logit")
def _(fig):
    ax = fig.subplots()
    fig.subplots_adjust(left=0.2)
    p = np.linspace(0.001, 0.999, 100)
    ax.plot(np.linspace(-7, 7, 100), p)
    ax.set_yscale("logit")


@case("axes/xscale_asinh", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(-500, 500, 101)
    ax.plot(x, np.arcsinh(x))
    ax.set_xscale("asinh")


@case("axes/xscale_function_sqrt", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 100, 50)
    ax.plot(x, x)
    ax.set_xscale("function", functions=(np.sqrt, np.square))


# --- tick formatting -------------------------------------------------------


@case("axes/ticklabel_format_sci", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, T * 1234.5)
    ax.ticklabel_format(axis="y", style="sci", scilimits=(0, 0))


@case("axes/ticklabel_format_mathtext", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T * 1e-5, T * 3e6)
    ax.ticklabel_format(style="sci", scilimits=(-2, 2), useMathText=True)


@case("axes/default_sci_large_values", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, T * 1e9)


@case("axes/large_offset_data", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, 1e5 + np.sin(T) * 0.01)


@case("axes/large_offset_use_offset_false", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T, 1e5 + np.sin(T) * 0.01)
    ax.ticklabel_format(useOffset=False)
    fig.subplots_adjust(left=0.25)


@case("axes/tiny_range", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot(T * 1e-9, np.sin(T) * 1e-9)


@case("axes/epoch_seconds_values", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    t = 1.7e9 + np.arange(0, 86400, 3600)
    ax.plot(t, np.cos(np.arange(24) / 4))
