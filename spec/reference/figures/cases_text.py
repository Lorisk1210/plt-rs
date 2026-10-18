"""Text: alignment, rotation, multi-line layout, fonts, colours, bbox
boxstyles, wrapping, coordinate systems, annotate (coords, arrow styles,
connection styles, shrink), mathtext under every bundled fontset, unicode,
path effects and tables."""

import numpy as np
from matplotlib import colormaps as mpl_colormaps
from matplotlib import patches
from matplotlib import patheffects as pe

from spec_support import case

BOXSTYLES = ["square", "round", "round4", "sawtooth", "roundtooth",
             "circle", "ellipse", "darrow", "larrow", "rarrow"]

LINE_ARROWSTYLES = ["-", "->", "<-", "<->", "-[", "]-", "]-[", "|-|", "]->", "<-["]
FILLED_ARROWSTYLES = ["-|>", "<|-", "<|-|>", "simple", "fancy", "wedge"]

MATH_SAMPLES = [
    r"$\frac{a}{b} + \dfrac{x^2}{y_1} - \sqrt{2} \cdot \sqrt[3]{x+1} \binom{n}{k} \frac{\frac{1}{2}}{z}$",
    r"$\sum_{i=0}^{\infty} x_i \quad \int_0^1 f(x)\,dx \quad \prod_{k=1}^{n} k \quad \oint_C \quad \lim_{x \to 0}$",
    r"$\hat{a}\ \bar{b}\ \vec{c}\ \dot{d}\ \ddot{e}\ \tilde{f}\ \acute{g}\ \breve{i}\ \widehat{xyz}\ \overline{AB}$",
    r"$\left( \frac{1}{2} \right) \left[ \frac{a}{b} \right] \left\{ \frac{\alpha}{\beta} \right\} \left| x \right| \left\langle \psi \right\rangle$",
    r"$a\!b\ |\ a\,b\ |\ a\:b\ |\ a\;b\ |\ a\quad b\ |\ a\qquad b$",
    r"$\mathrm{Rm}\ \mathit{It}\ \mathbf{Bf}\ \mathcal{CAL}\ \mathbb{RNZ}\ \mathfrak{Frak}\ \mathsf{Sf}\ \mathtt{Tt}$",
    r"$\alpha\beta\gamma\delta\epsilon\theta\lambda\pi\Gamma\Delta\Omega\ \leq\geq\neq\approx\infty\partial\nabla\pm\times\rightarrow\Leftrightarrow$",
    r"$x_{i,j}^{2n+1}\ e^{-x^2/2}\ 10^{-3}\ {}^{14}\mathrm{C}\ \mathrm{sin}^2\theta\ \operatorname{erf}(x)\ \underline{u}$",
]

LOREM = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor "
         "incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud "
         "exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.")


def _blank(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    return ax


# --- alignment, rotation, layout

@case("text/text_default")
def _(fig):
    ax = fig.subplots()
    ax.text(0.5, 0.5, "default text")
    ax.text(0.1, 0.8, "second\nline")


@case("text/ha_va_grid", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    vas = {"top": "top", "center": "ctr", "baseline": "base", "bottom": "bot",
           "center_baseline": "cbase"}
    for i, ha in enumerate(["left", "center", "right"]):
        for j, va in enumerate(vas):
            x, y = 1.5 + 3.5 * i, 9 - 2 * j
            ax.plot(x, y, "r+", ms=10)
            ax.text(x, y, f"{ha[0]}/{vas[va]}g", ha=ha, va=va, fontsize=11)


@case("text/rotation_default_mode", figsize=(6, 3))
def _(fig):
    ax = _blank(fig)
    for i, rot in enumerate([0, 30, 45, 90, 135, 180, 270]):
        x = 0.8 + 1.4 * i
        ax.plot(x, 5, "r+", ms=10)
        ax.text(x, 5, f"rot {rot}", rotation=rot, ha="left", va="bottom")


@case("text/rotation_mode_anchor", figsize=(6, 3))
def _(fig):
    ax = _blank(fig)
    for i, rot in enumerate([0, 30, 45, 90, 135, 180, 270]):
        x = 0.8 + 1.4 * i
        ax.plot(x, 5, "r+", ms=10)
        ax.text(x, 5, f"rot {rot}", rotation=rot, rotation_mode="anchor", ha="left", va="bottom")


@case("text/multialignment_linespacing", figsize=(6, 3))
def _(fig):
    ax = _blank(fig)
    s = "short\na much longer line\nmid line"
    ax.text(0.5, 5, s, multialignment="left", va="center", bbox=dict(fc="none"))
    ax.text(5, 5, s, multialignment="center", ha="center", va="center", linespacing=2.0,
            bbox=dict(fc="none"))
    ax.text(9.5, 5, s, multialignment="right", ha="right", va="center", linespacing=0.9,
            bbox=dict(fc="none"))


@case("text/font_sizes", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    names = ["xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large"]
    for i, size in enumerate(names):
        ax.text(0.3, 9.3 - 1.3 * i, f"size {size}", fontsize=size)
    for i, size in enumerate([6, 9, 14, 20, 28]):
        ax.text(5.5, 9.3 - 1.9 * i, f"{size} pt", fontsize=size)


@case("text/font_families_styles", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    rows = [("sans-serif", "normal"), ("serif", "normal"), ("monospace", "normal"),
            ("sans-serif", "italic"), ("serif", "italic"), ("monospace", "oblique"),
            ("DejaVu Serif", "normal")]
    for i, (family, style) in enumerate(rows):
        ax.text(0.5, 9.2 - 1.3 * i, f"{family} {style} Quick fox 0123", family=family,
                style=style, fontsize=13)


@case("text/font_weights_stretch", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    weights = ["ultralight", "light", "normal", "medium", "semibold", "bold", "heavy", "black"]
    for i, weight in enumerate(weights):
        ax.text(0.3, 9.3 - 1.15 * i, f"weight {weight}", weight=weight, fontsize=12)
    for i, stretch in enumerate(["ultra-condensed", "condensed", "normal", "expanded"]):
        ax.text(5.3, 9.3 - 1.5 * i, f"{stretch}", fontstretch=stretch, fontsize=12)
    ax.text(5.3, 2.5, "serif bold", family="serif", weight="bold", fontsize=12)
    ax.text(5.3, 1.0, "mono bold", family="monospace", weight="bold", fontsize=12)


@case("text/color_alpha_background", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    ax.axhspan(4, 6, color="k")
    ax.text(0.5, 8, "red text", color="red", fontsize=14)
    ax.text(0.5, 4.6, "half transparent", color="tab:blue", alpha=0.5, fontsize=16)
    ax.text(5.5, 4.6, "opaque", color="tab:blue", fontsize=16)
    ax.text(0.5, 1.5, "background colour", backgroundcolor="yellow", fontsize=12)


@case("text/rc_text_color", figsize=(4, 3.5), rc={"text.color": "tab:green"})
def _(fig):
    ax = fig.subplots()
    ax.text(0.2, 0.5, "inherits text.color")
    ax.set_title("title colour")
    ax.set_xlabel("label (axes.labelcolor)")
    fig.subplots_adjust(bottom=0.18)


@case("text/bbox_boxstyles", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    for i, style in enumerate(BOXSTYLES):
        x, y = 2 + 6 * (i % 2), 9 - 2 * (i // 2)
        ax.text(x, y, style, ha="center", va="center", size=11,
                bbox=dict(boxstyle=style, fc="lightsteelblue", ec="navy"))


@case("text/bbox_pad_props", figsize=(6, 3))
def _(fig):
    ax = _blank(fig)
    ax.text(1, 7, "pad 0.1", bbox=dict(boxstyle="square,pad=0.1", fc="w"))
    ax.text(5, 7, "pad 1.0", bbox=dict(boxstyle="square,pad=1.0", fc="w"))
    ax.text(1, 2, "round size", bbox=dict(boxstyle="round,pad=0.6,rounding_size=0.8",
                                         fc="gold", ec="k", lw=2))
    ax.text(5, 2, "alpha + dash", bbox=dict(fc="tab:red", alpha=0.3, ls="--", ec="k"))
    ax.text(8.5, 5, "rotated", rotation=30, bbox=dict(boxstyle="larrow", fc="0.9"))


@case("text/wrap", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.text(0.05, 0.95, LOREM, wrap=True, va="top", transform=ax.transAxes, fontsize=9)
    ax.text(0.05, 0.1, LOREM[:120], wrap=True, rotation=15, fontsize=9, transform=ax.transAxes)


@case("text/fig_text_wrap_paragraph", figsize=(5, 3))
def _(fig):
    fig.text(0.02, 0.98, LOREM + " " + LOREM, wrap=True, va="top", usetex=False, fontsize=10)
    fig.text(0.5, 0.1, "centred: " + LOREM[:90], wrap=True, ha="center", usetex=False)


@case("text/coordinate_systems", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(0, 4)
    ax.set_ylim(0, 2)
    ax.text(1, 1, "data (1, 1)")
    ax.text(0.98, 0.02, "axes (0.98, 0.02)", transform=ax.transAxes, ha="right")
    ax.text(0.5, 1.02, "axes top", transform=ax.transAxes, ha="center")
    fig.text(0.01, 0.01, "figure (0.01, 0.01)")
    fig.text(0.99, 0.99, "fig top right", ha="right", va="top")
    fig.suptitle("suptitle")


# --- annotate

@case("text/annotate_coord_kinds", figsize=(6, 4.5))
def _(fig):
    ax = fig.subplots()
    ax.set_xlim(-2, 2)
    ax.set_ylim(-2, 2)
    arrow = dict(arrowstyle="->")
    ax.annotate("data", xy=(0.5, 0.5), xytext=(-1.5, 1.5), arrowprops=arrow)
    ax.annotate("axes fraction", xy=(0.1, 0.1), xycoords="axes fraction", xytext=(0.3, 0.3),
                textcoords="axes fraction", arrowprops=arrow)
    ax.annotate("figure pixels", xy=(5, 5), xycoords="figure pixels")
    ax.annotate("figure fraction", xy=(0.7, 0.92), xycoords="figure fraction")
    ax.annotate("offset points", xy=(1, -1), xytext=(-30, -25), textcoords="offset points",
                arrowprops=arrow)
    ax.annotate("offset pixels", xy=(1.5, 1), xytext=(-60, 20), textcoords="offset pixels",
                arrowprops=arrow)
    ax.annotate("polar", xy=(np.pi / 4, 1.5), xycoords="polar", xytext=(-1.8, -0.5),
                textcoords="data", arrowprops=arrow)
    ax.annotate("polar text", xy=(-1, -1.5), xytext=(3 * np.pi / 4, 1.8), textcoords="polar",
                arrowprops=arrow)
    ax.annotate("offset fontsize", xy=(0, 0), xytext=(2, -4), textcoords="offset fontsize",
                arrowprops=arrow)
    ax.plot([0.5, 1, 1.5, 0], [0.5, -1, 1, 0], "k.")


@case("text/annotate_axes_coords_and_artist", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    line, = ax.plot([0, 1, 2], [0, 1, 0])
    t = ax.annotate("peak", xy=(1, 1), xytext=(0.2, 0.5), textcoords="axes fraction",
                    arrowprops=dict(arrowstyle="->"), bbox=dict(boxstyle="round", fc="w"))
    ax.annotate("from text", xy=(1, 0), xycoords=t, xytext=(0, -40), textcoords="offset points",
                ha="left", arrowprops=dict(arrowstyle="<-"))
    ax.annotate("outside xlim", xy=(2.4, 0.5), xytext=(0.6, 0.85), textcoords="axes fraction",
                arrowprops=dict(arrowstyle="->"), annotation_clip=False)


@case("text/arrowstyles_line", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    for i, style in enumerate(LINE_ARROWSTYLES):
        x, y = 0.5 + 5 * (i // 5), 9 - 2 * (i % 5)
        ax.annotate(style, xy=(x + 3, y), xytext=(x, y), va="center",
                    arrowprops=dict(arrowstyle=style, mutation_scale=15))


@case("text/arrowstyles_filled", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    for i, style in enumerate(FILLED_ARROWSTYLES):
        x, y = 0.5 + 5 * (i // 3), 8.5 - 3 * (i % 3)
        ax.annotate(style, xy=(x + 3.5, y), xytext=(x, y), va="center",
                    arrowprops=dict(arrowstyle=style, mutation_scale=20, fc="tab:orange",
                                    ec="k"))


@case("text/arrowstyle_params", figsize=(6, 4))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    styles = ["->,head_length=0.8,head_width=0.4", "-[,widthB=1.5,lengthB=0.5",
              "|-|,widthA=0.3,widthB=1.0", "-|>,head_length=1.0,head_width=0.2",
              "fancy,head_length=0.6,head_width=0.6,tail_width=0.3",
              "simple,head_length=0.3,head_width=0.8,tail_width=0.6",
              "wedge,tail_width=0.7,shrink_factor=0.3", "<->,head_length=0.2,head_width=0.6"]
    for i, style in enumerate(styles):
        x, y = 0.2 + 5 * (i // 4), 9 - 2.5 * (i % 4)
        ax.annotate(style.split(",")[0], xy=(x + 4, y), xytext=(x, y), va="center",
                    arrowprops=dict(arrowstyle=style, mutation_scale=15, fc="w"))


@case("text/connectionstyles", figsize=(8, 5))
def _(fig):
    axs = fig.subplots(2, 4).flat
    fig.subplots_adjust(hspace=0.6, wspace=0.3)
    styles = ["arc3", "arc3,rad=0.3", "arc3,rad=-0.3", "angle3,angleA=90,angleB=0",
              "angle,angleA=-90,angleB=180,rad=0", "angle,angleA=-90,angleB=180,rad=5",
              "arc,angleA=-90,angleB=0,armA=30,armB=30,rad=0", "bar,fraction=0.1"]
    for ax, style in zip(axs, styles):
        ax.plot([0.2, 0.8], [0.2, 0.8], "ko", ms=3)
        ax.annotate("", xy=(0.2, 0.2), xytext=(0.8, 0.8),
                    arrowprops=dict(arrowstyle="->", connectionstyle=style, shrinkA=4, shrinkB=4))
        ax.set_title(style.replace(",", "\n"), fontsize=7)
        ax.set_xticks([])
        ax.set_yticks([])


@case("text/shrink_and_patches", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    for i, (sa, sb) in enumerate([(0, 0), (5, 5), (15, 25)]):
        y = 8 - 3 * i
        ax.plot([1, 8], [y, y], "r.")
        ax.annotate(f"shrinkA={sa} shrinkB={sb}", xy=(8, y), xytext=(1, y), va="bottom",
                    arrowprops=dict(arrowstyle="->", shrinkA=sa, shrinkB=sb))
    ax.annotate("boxed", xy=(9, 1), xytext=(6, 2.5), bbox=dict(boxstyle="round", fc="w"),
                arrowprops=dict(arrowstyle="-|>", connectionstyle="arc3,rad=0.4"))


@case("text/annotate_legacy_arrowprops", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    ax.annotate("default dict", xy=(8, 8), xytext=(1, 7), arrowprops=dict())
    ax.annotate("width/headwidth", xy=(8, 4), xytext=(1, 3),
                arrowprops=dict(width=4, headwidth=12, headlength=10, shrink=0.05,
                                facecolor="tab:green"))


# --- mathtext under every bundled fontset

def _math_case(fontset):
    def build(fig):
        for i, s in enumerate(MATH_SAMPLES):
            fig.text(0.02, 0.97 - 0.12 * i, s, fontsize=13, va="top")

    return build


for _fontset in ["dejavusans", "dejavuserif", "cm", "stix", "stixsans"]:
    case(f"text/mathtext_{_fontset}", figsize=(7, 6),
         rc={"mathtext.fontset": _fontset})(_math_case(_fontset))


@case("text/mathtext_title_labels", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0.0, 2.0, 30)
    ax.plot(x, np.exp(-(x**2)), label=r"$e^{-x^2}$")
    ax.set_title(r"$\alpha_i > \beta_i \quad \sum_{n=1}^\infty \frac{1}{n^2} = \frac{\pi^2}{6}$")
    ax.set_xlabel(r"time $t$ [$\mu$s]")
    ax.set_ylabel(r"$\sqrt{\langle x^2 \rangle}$")
    ax.set_xticks([0, 1, 2], [r"$0$", r"$\frac{1}{2}\pi$", r"$\pi$"])
    ax.legend()


@case("text/mathtext_mixed_sizes", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    for i, size in enumerate([8, 12, 18, 26]):
        ax.text(0.2, 9 - 2.4 * i, r"Area $A = \pi r^2$, $\int_a^b g$", fontsize=size, va="top")


@case("text/unicode", figsize=(6, 3))
def _(fig):
    ax = _blank(fig)
    ax.set_axis_off()
    ax.text(0.2, 8.5, "Greek: αβγδε ΓΔΘΛΞΠΣΦΨΩ", fontsize=13)
    ax.text(0.2, 6.5, "Latin: àáâãäå çèéêë ñ øœ ß ÀÉÎÕÜ Łódź", fontsize=13)
    ax.text(0.2, 4.5, "Arrows: ← → ↑ ↓ ↔ ⇐ ⇒ ⇔", fontsize=13)
    ax.text(0.2, 2.5, "Symbols: ∞ ± × ÷ ≤ ≥ ≠ ≈ ∑ ∫ √ °C € £ ¥", fontsize=13)
    ax.text(0.2, 0.5, "Cyrillic: Привет мир", fontsize=13, family="serif")


# --- path effects

@case("text/patheffects_withstroke", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.imshow(np.arange(100).reshape(10, 10), cmap="gray", extent=(0, 10, 0, 10))
    ax.text(1, 6, "stroked text", fontsize=24, color="w",
            path_effects=[pe.withStroke(linewidth=4, foreground="k")])
    ax.plot([0, 10], [2, 4], color="w", lw=2, path_effects=[pe.withStroke(linewidth=6, foreground="r")])


@case("text/patheffects_stroke_normal", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    ax.text(1, 6, "Stroke + Normal", fontsize=24, color="gold",
            path_effects=[pe.Stroke(linewidth=5, foreground="navy"), pe.Normal()])
    ax.plot([1, 9], [2, 3], lw=3, path_effects=[pe.Stroke(linewidth=8, foreground="0.7"), pe.Normal()])


@case("text/patheffects_shadows", figsize=(5, 3))
def _(fig):
    ax = _blank(fig)
    ax.text(0.5, 7, "simple shadow", fontsize=22, path_effects=[pe.withSimplePatchShadow()])
    ax.plot([1, 9], [5, 5.5], lw=4, path_effects=[pe.SimpleLineShadow(), pe.Normal()])
    rect = patches.Rectangle((1, 1), 3, 2, fc="tab:blue",
                             path_effects=[pe.SimplePatchShadow(offset=(4, -4), shadow_rgbFace="k",
                                                                alpha=0.4), pe.Normal()])
    ax.add_patch(rect)
    ax.text(6, 1.5, "offset", fontsize=18,
            path_effects=[pe.withSimplePatchShadow(offset=(3, -3), shadow_rgbFace="r", alpha=0.6)])


@case("text/patheffects_ticked_stroke", figsize=(4, 3))
def _(fig):
    ax = _blank(fig)
    ax.plot([1, 9], [2, 8], path_effects=[pe.withTickedStroke(angle=135, spacing=10, length=1)])
    theta = np.linspace(0.0, 2 * np.pi, 60)
    ax.plot(5 + 2 * np.cos(theta), 5 + 2 * np.sin(theta), color="tab:red",
            path_effects=[pe.withTickedStroke()])


# --- tables

def _table_cells():
    return [["1.0", "2.5", "3.25"], ["10", "20", "30"], ["a", "bb", "ccc"]]


@case("text/table_bottom", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2], [1, 3, 2])
    ax.table(cellText=_table_cells(), rowLabels=["r1", "r2", "r3"],
             colLabels=["A", "B", "C"], loc="bottom")
    fig.subplots_adjust(bottom=0.3)
    ax.set_xticks([])


@case("text/table_loc_variants", figsize=(6, 4))
def _(fig):
    ax = fig.subplots()
    for loc in ["upper left", "upper right", "center", "lower left", "lower right"]:
        ax.table(cellText=[[loc], ["x"]], loc=loc, colWidths=[0.25])


@case("text/table_colours_widths_fontsize", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    cells = [["#fdd", "#dfd", "#ddf"]] * 3
    table = ax.table(cellText=_table_cells(), cellColours=cells, rowLabels=["r1", "r2", "r3"],
                     rowColours=["0.8"] * 3, colLabels=["A", "B", "C"], colColours=["gold"] * 3,
                     colWidths=[0.15, 0.3, 0.2], cellLoc="right", loc="center")
    table.auto_set_font_size(False)
    table.set_fontsize(12)
    table.scale(1.0, 1.6)


@case("text/table_edges_and_auto_width", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    table = ax.table(cellText=[["short", "a much longer cell"], ["x", "y"]],
                     colLabels=["one", "two"], edges="horizontal", loc="upper center")
    table.auto_set_column_width([0, 1])
    ax.table(cellText=[["open", "closed"]], edges="BT", loc="lower center")


@case("text/table_cell_styles_header", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    rows = [["Mon", "12", "3.5"], ["Tue", "15", "4.0"], ["Wed", "9", "2.25"], ["Thu", "21", "5.5"]]
    table = ax.table(cellText=rows, colLabels=["day", "count", "mean"], loc="center")
    for (row, col), cell in table.get_celld().items():
        cell.set_edgecolor("0.7")
        if row == 0:
            cell.set_facecolor("#40466e")
            cell.set_text_props(weight="bold", color="w")
            cell.set_linewidth(2)
        elif row % 2 == 0:
            cell.set_facecolor("#f1f1f2")
    table[(4, 1)].set_text_props(color="tab:red")


@case("text/table_visible_edges_per_cell", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    table = ax.table(cellText=[["open", "B", "LR"], ["T", "closed", "BRT"]], loc="center",
                     cellLoc="center")
    for (row, col), cell in table.get_celld().items():
        cell.visible_edges = cell.get_text().get_text()
    table.scale(1, 2)


@case("text/table_outside_locations", figsize=(6, 4.5))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1])
    for loc in ["top", "left", "right", "bottom right", "top left"]:
        ax.table(cellText=[[loc]], loc=loc, colWidths=[0.2])
    fig.subplots_adjust(left=0.25, right=0.75, top=0.8, bottom=0.2)


@case("text/table_auto_font_size_shrinks", figsize=(6, 3))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    cells = [["a fairly long entry", "x"], ["short", "another long entry"]]
    for ax in (ax1, ax2):
        ax.set_axis_off()
    ax1.table(cellText=cells, colWidths=[0.4, 0.4], loc="center")
    fixed = ax2.table(cellText=cells, colWidths=[0.4, 0.4], loc="center")
    fixed.auto_set_font_size(False)
    fixed.set_fontsize(7)


@case("text/table_bbox_cell_locs", figsize=(5, 4))
def _(fig):
    ax = fig.subplots()
    ax.set_xticks([])
    ax.set_yticks([])
    ax.table(cellText=[["1", "22", "333"], ["4444", "5", "66"]], rowLabels=["first", "second"],
             colLabels=["A", "B", "C"], cellLoc="left", rowLoc="center", colLoc="right",
             bbox=[0.25, 0.2, 0.7, 0.5])


@case("text/table_colours_only_scaled", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    values = np.linspace(0.0, 1.0, 12).reshape(3, 4)
    colours = [[mpl_colormaps["viridis"](v) for v in row] for row in values]
    table = ax.table(cellColours=colours, rowLabels=["r1", "r2", "r3"],
                     colLabels=["w", "x", "y", "z"], loc="center")
    table.scale(1, 2.5)
    for col in range(4):
        table[(0, col)].visible_edges = "B"
    table[(1, 2)].set_height(0.2)


@case("text/table_cell_size_text_edit", figsize=(5, 3))
def _(fig):
    ax = fig.subplots()
    ax.set_axis_off()
    table = ax.table(cellText=[["a", "b"], ["c", "d"]], loc="center", colWidths=[0.2, 0.2])
    table[(0, 0)].get_text().set_text("edited")
    table[(0, 1)].set_width(0.35)
    table[(1, 0)].set_text_props(fontstyle="italic", ha="left")
    table[(1, 1)].set_facecolor("tab:olive")
    table[(1, 1)].set_alpha(0.5)
