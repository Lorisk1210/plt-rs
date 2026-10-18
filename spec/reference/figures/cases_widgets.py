"""Widgets drawn in their default and styled states (initial rendering
only): check and radio buttons with the 3.7 ``label_props``,
``frame_props``, ``check_props`` and ``radio_props`` and the 3.11
``layout``, sliders and range sliders with ``handle_style`` and patch
keywords, buttons and text boxes."""

from matplotlib import widgets

from spec_support import case

COLORS = ["tab:red", "tab:green", "tab:blue", "k"]


# --- check and radio buttons


@case("widgets/check_buttons_styled_props", figsize=(4, 3))
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    check = widgets.CheckButtons(
        ax, ["red", "green", "blue", "black"], [True, False, True, False],
        label_props={"color": COLORS, "fontsize": [8, 10, 12, 14]},
        frame_props={"edgecolor": COLORS, "facecolor": "0.95", "s": [40, 60, 80, 100],
                     "linewidth": 2},
        check_props={"facecolor": COLORS, "linewidth": 2})
    del check


@case("widgets/radio_buttons_styled_props", figsize=(4, 3))
def _(fig):
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8), facecolor="ivory")
    radio = widgets.RadioButtons(
        ax, ["slow", "medium", "fast"], active=2,
        label_props={"color": ["0.4", "k", "tab:red"], "fontsize": [9, 11, 13],
                     "fontweight": ["normal", "normal", "bold"]},
        radio_props={"facecolor": ["tab:blue", "tab:orange", "tab:red"], "edgecolor": "0.3",
                     "s": [30, 60, 120]})
    del radio


@case("widgets/radio_buttons_activecolor_layouts", figsize=(6, 3))
def _(fig):
    across = widgets.RadioButtons(fig.add_axes((0.05, 0.55, 0.9, 0.35)),
                                  ["one", "two", "three", "four"], active=1,
                                  activecolor="tab:green", layout="horizontal")
    grid = widgets.RadioButtons(fig.add_axes((0.05, 0.1, 0.4, 0.35)),
                                ["a", "bb", "ccc", "dddd", "e"], active=3, layout=(2, 3))
    column = widgets.RadioButtons(fig.add_axes((0.55, 0.1, 0.4, 0.35)),
                                  ["top", "middle", "bottom"], layout="vertical")
    del across, grid, column


@case("widgets/check_buttons_layouts", figsize=(6, 3))
def _(fig):
    across = widgets.CheckButtons(fig.add_axes((0.05, 0.55, 0.9, 0.35)),
                                  ["alpha", "beta", "gamma"], [True, False, True],
                                  layout="horizontal")
    grid = widgets.CheckButtons(fig.add_axes((0.05, 0.1, 0.9, 0.35)), list("ABCDEF"),
                                [i % 2 == 0 for i in range(6)], layout=(2, 3),
                                label_props={"fontsize": [9]})
    del across, grid


# --- sliders


@case("widgets/slider_handle_style_poly_props", figsize=(5, 3))
def _(fig):
    hz = widgets.Slider(fig.add_axes((0.25, 0.75, 0.55, 0.1)), "Hz", 0, 50, valinit=12.5,
                        valfmt="%1.1f Hz",
                        handle_style={"facecolor": "tab:orange", "edgecolor": "k", "size": 14},
                        track_color="0.92", initcolor="none", facecolor="tab:green", alpha=0.6)
    gain = widgets.Slider(fig.add_axes((0.25, 0.5, 0.55, 0.1)), "Gain", -10, 10, valinit=-3,
                          valstep=[-10, -6, -3, 0, 3, 6, 10], facecolor="tab:purple",
                          initcolor="tab:blue", edgecolor="k", linewidth=1, hatch="//")
    big = widgets.Slider(fig.add_axes((0.25, 0.25, 0.55, 0.1)), "Big", 0, 2e6, valinit=7.5e5)
    del hz, gain, big


@case("widgets/slider_vertical_closed_ends", figsize=(4, 4))
def _(fig):
    lo = widgets.Slider(fig.add_axes((0.15, 0.15, 0.08, 0.7)), "lo", 0, 1, valinit=0,
                        closedmin=False, orientation="vertical")
    mid = widgets.Slider(fig.add_axes((0.45, 0.15, 0.08, 0.7)), "mid", 0, 100, valinit=37,
                         valstep=10, orientation="vertical", track_color="lightblue",
                         handle_style={"size": 6})
    hi = widgets.Slider(fig.add_axes((0.75, 0.15, 0.08, 0.7)), "hi", -1, 1, valinit=2,
                        orientation="vertical", initcolor="none")
    del lo, mid, hi


@case("widgets/range_slider_styles", figsize=(5, 4))
def _(fig):
    default = widgets.RangeSlider(fig.add_axes((0.25, 0.8, 0.6, 0.08)), "default", 0, 10)
    styled = widgets.RangeSlider(fig.add_axes((0.25, 0.6, 0.6, 0.08)), "styled", -5, 5,
                                 valinit=(-4, 1), valfmt="%+.1f",
                                 handle_style={"facecolor": "tab:red", "size": 8},
                                 facecolor="tab:red", alpha=0.4, track_color="mistyrose")
    swapped = widgets.RangeSlider(fig.add_axes((0.25, 0.4, 0.6, 0.08)), "swapped", 0, 1,
                                  valinit=(0.9, 0.2))
    vertical = widgets.RangeSlider(fig.add_axes((0.08, 0.1, 0.06, 0.4)), "vert", 0, 1,
                                   valinit=(0.1, 0.4), orientation="vertical", valstep=0.05,
                                   edgecolor="k")
    del default, styled, swapped, vertical


# --- buttons and text boxes


@case("widgets/button_styles", figsize=(5, 2))
def _(fig):
    plain = widgets.Button(fig.add_axes((0.05, 0.3, 0.25, 0.4)), "Default")
    styled = widgets.Button(fig.add_axes((0.375, 0.3, 0.25, 0.4)), "Styled", color="tab:blue",
                            hovercolor="tab:cyan")
    styled.label.set_color("w")
    styled.label.set_fontweight("bold")
    styled.label.set_fontsize(14)
    framed = widgets.Button(fig.add_axes((0.7, 0.3, 0.25, 0.4)), "Framed\ntwo lines",
                            color="lightyellow")
    for spine in framed.ax.spines.values():
        spine.set_edgecolor("tab:orange")
        spine.set_linewidth(2)
    del plain


@case("widgets/textbox_alignments_colors", figsize=(5, 3))
def _(fig):
    left = widgets.TextBox(fig.add_axes((0.25, 0.75, 0.6, 0.12)), "left ",
                           initial="aligned left")
    centre = widgets.TextBox(fig.add_axes((0.25, 0.5, 0.6, 0.12)), "center ", initial="centred",
                             textalignment="center", color="lightcyan")
    right = widgets.TextBox(fig.add_axes((0.25, 0.25, 0.6, 0.12)), "right",
                            initial="right $x^2$", textalignment="right", label_pad=0.05,
                            hovercolor="w")
    empty = widgets.TextBox(fig.add_axes((0.25, 0.05, 0.6, 0.12)), "empty", initial="")
    del left, centre, right, empty
