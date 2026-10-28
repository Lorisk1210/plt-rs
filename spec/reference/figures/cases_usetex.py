r"""Text typeset by LaTeX (``text.usetex``), drawn from the DVI file with
matplotlib 3.11's default ``text.latex.engine`` (``latex``) unless a case
says ``latex+dvipng``. Every case needs TeX and is skipped without it.

* Text, display math, multi-line text, rotation in every alignment
  (``rotation``, matplotlib's ``test_usetex.py::test_rotation``), tick
  labels (linear, log, dates), titles and labels, legends, annotations,
  colorbars, tight and constrained layout, path effects.
* The font families TeX selects from ``font.family`` and the family lists
  (Computer Modern, Helvetica, Palatino, Times, Courier, Zapf Chancery),
  through PSNFSS's virtual fonts, and ``text.latex.preamble``.
* Ports of ``tests/test_usetex.py`` (``test_usetex``, ``test_empty``,
  ``test_unicode_minus``, ``test_mathdefault``, ``test_multiline_eqnarray``,
  ``test_usetex_with_underscore``, ``test_latex_pkg_already_loaded``,
  ``test_usetex_packages``) and the usetex parts of other test files:
  ``test_backend_pdf.py`` (``test_failing_latex`` aside, the kerning texts
  at 50 pt), ``test_backend_svg.py`` (``\textwon``), ``test_text.py``
  (multi-line usetex, ``test_single_artist_usetex``), ``test_axes.py``
  (``test_pie_textprops`` with usetex).
"""

import matplotlib.patheffects as path_effects
import numpy as np

from spec_support import case

GROUP = "usetex"
TEX = {"text.usetex": True}


def usetex_case(name, *, figsize=(3.2, 2.0), dpi=100, rc=None):
    """Registers ``usetex/<name>`` with ``text.usetex`` on, needing TeX."""
    return case(f"{GROUP}/{name}", figsize=figsize, dpi=dpi, rc={**TEX, **(rc or {})},
                needs_tex=True)


@usetex_case("text_basic")
def _(fig):
    fig.text(0.05, 0.8, "Plain usetex text, 10 pt")
    fig.text(0.05, 0.6, r"Inline math $\alpha^2 + \beta_i = \gamma$", fontsize=12)
    fig.text(0.05, 0.4, r"\textbf{Bold} \textit{italic} \texttt{typewriter}", fontsize=9)
    fig.text(0.05, 0.15, r"Large $\sum_{n=1}^{\infty} \frac{1}{n^2}$", fontsize=16)


@usetex_case("math_display", figsize=(3.2, 2.4))
def _(fig):
    fig.text(0.5, 0.75, r"$\displaystyle\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}$",
             ha="center", fontsize=12)
    fig.text(0.5, 0.4, r"$\left[\frac{\partial^2 u}{\partial t^2}\right] = c^2 \nabla^2 u$",
             ha="center", va="center", fontsize=12)
    fig.text(0.5, 0.1, r"$\sqrt[3]{x} \ne \overline{y}$, $-1$, $10^{-3}$", ha="center")


@usetex_case("multiline", figsize=(3.2, 2.0))
def _(fig):
    fig.text(0.3, 0.5, "foo\nbar", ha="center", va="center")
    fig.text(0.7, 0.5, "first line\n\n$x^2$ after a blank", ha="right", va="center",
             multialignment="center")
    fig.text(0.5, 0.1, " ", ha="center")


@usetex_case("rotation", figsize=(6.4, 4.8))
def _(fig):
    # matplotlib's test_usetex.py::test_rotation.
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set(xlim=(-0.5, 5), xticks=[], ylim=(-0.5, 3), yticks=[], frame_on=False)
    text = {val: val[0] for val in ["top", "center", "bottom", "left", "right"]}
    text["baseline"] = "B"
    text["center_baseline"] = "C"
    for i, va in enumerate(["top", "center", "bottom", "baseline", "center_baseline"]):
        for j, ha in enumerate(["left", "center", "right"]):
            for k, angle in enumerate([0, 90, 180, 270]):
                k //= 2
                x = i + k / 2
                y = j + k / 2
                ax.plot(x, y, "+", c=f"C{k}", markersize=20, markeredgewidth=0.5)
                ax.text(x, y, f"$\\mathrm{{My {text[ha]}{text[va]} {angle}}}$",
                        rotation=angle, horizontalalignment=ha, verticalalignment=va)


@usetex_case("rotation_anchor", figsize=(3.2, 2.4))
def _(fig):
    for angle, x in zip([15, 45, 120, 200], [0.15, 0.4, 0.65, 0.85]):
        fig.text(x, 0.5, rf"$\theta = {angle}^\circ$", rotation=angle,
                 rotation_mode="anchor", ha="center", va="center")


@usetex_case("ticks", figsize=(4.0, 3.0))
def _(fig):
    ax1, ax2 = fig.subplots(1, 2)
    x = np.linspace(-2.0, 2.0, 21)
    ax1.plot(x, x ** 3 * 1e4)
    ax2.semilogy(x + 2.5, np.exp(x * 2.0))


@usetex_case("date_ticks", figsize=(4.0, 2.0))
def _(fig):
    import datetime

    import matplotlib.dates as mdates

    ax = fig.subplots()
    start = datetime.datetime(2020, 1, 1)
    days = [start + datetime.timedelta(days=i) for i in range(0, 60, 6)]
    ax.plot(days, np.arange(len(days)))
    locator = mdates.AutoDateLocator()
    ax.xaxis.set_major_locator(locator)
    ax.xaxis.set_major_formatter(mdates.ConciseDateFormatter(locator))


@usetex_case("titles_labels", figsize=(4.0, 3.0))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1, 2], [0, 1, 4])
    ax.set_title(r"Title with $\mathcal{L}$", loc="left")
    ax.set_title(r"right", loc="right", fontsize=8)
    ax.set_xlabel(r"time $t$ (s)")
    ax.set_ylabel(r"$\|v\|^2$")
    fig.suptitle(r"\textsc{Suptitle}")


@usetex_case("legend", figsize=(3.2, 2.4))
def _(fig):
    # matplotlib's test_usetex_with_underscore, plus labels with math.
    ax = fig.subplots()
    df = {"a_b": range(5)[::-1], "c": range(5)}
    ax.plot("c", "a_b", data=df)
    ax.plot([0, 4], [1, 3], label=r"$y = \frac{x}{2} + 1$")
    ax.legend(title=r"\emph{Legend}")
    ax.text(0, 0, "foo_bar", usetex=True)


@usetex_case("annotations", figsize=(3.2, 2.4))
def _(fig):
    ax = fig.subplots()
    ax.plot([0, 1], [0, 1])
    ax.annotate(r"$\max$ here", xy=(1, 1), xytext=(0.2, 0.7),
                arrowprops=dict(arrowstyle="->"), bbox=dict(boxstyle="round", fc="w"))
    ax.text(0.6, 0.2, r"boxed $\epsilon$", bbox=dict(pad=0, edgecolor="k", facecolor="none"))


@usetex_case("colorbar", figsize=(3.2, 2.4))
def _(fig):
    ax = fig.subplots()
    image = ax.imshow(np.arange(20.0).reshape(4, 5) / 7.0)
    fig.colorbar(image, ax=ax, label=r"$\rho$ (kg/m$^3$)")


@usetex_case("tight_layout", figsize=(4.0, 3.0))
def _(fig):
    axs = fig.subplots(2, 2)
    for i, ax in enumerate(axs.flat):
        ax.plot([0, 1], [0, i])
        ax.set_title(rf"Panel ${i}$")
        ax.set_xlabel(r"$x_{%d}$" % i)
        ax.set_ylabel(r"$\int f_{%d}$" % i)
    fig.tight_layout()


@usetex_case("constrained_layout", figsize=(4.0, 3.0), rc={"figure.constrained_layout.use": True})
def _(fig):
    axs = fig.subplots(1, 2)
    for i, ax in enumerate(axs):
        ax.plot([0, 1], [1, i])
        ax.set_title(r"\LaTeX\ %d" % i)
        ax.set_xlabel(r"$\xi$")
    fig.supxlabel(r"shared $x$")


@usetex_case("path_effects", figsize=(3.2, 1.6))
def _(fig):
    text = fig.text(0.1, 0.4, r"Stroke $\Omega$", fontsize=20)
    text.set_path_effects([path_effects.withStroke(linewidth=3, foreground="tab:orange")])


@usetex_case("dvipng_engine", figsize=(3.2, 2.0), rc={"text.latex.engine": "latex+dvipng"})
def _(fig):
    fig.text(0.05, 0.7, r"dvipng: $\frac{a}{b} + \sqrt{2}$", fontsize=12)
    fig.text(0.05, 0.3, r"rasterized by \texttt{dvipng}", color="tab:blue")
    fig.text(0.75, 0.15, r"turned $\int_0^1$", rotation=30)
    fig.text(0.95, 0.1, r"$x^2$ up", rotation=90)


@usetex_case("family_sans_helvetica", rc={"font.family": "sans-serif",
                                          "font.sans-serif": ["Helvetica"]})
def _(fig):
    fig.text(0.05, 0.6, r"Helvetica $x^2$ fi ffl", fontsize=12)
    fig.text(0.05, 0.2, r"\textrm{rm} \texttt{tt}", fontsize=12)


@usetex_case("family_serif_palatino", rc={"font.family": "serif", "font.serif": ["Palatino"]})
def _(fig):
    fig.text(0.05, 0.6, r"Palatino $\alpha + \beta$", fontsize=12)
    fig.text(0.05, 0.2, r"\textsf{sans} \textit{italic}", fontsize=12)


@usetex_case("family_times", rc={"font.family": "times"})
def _(fig):
    fig.text(0.05, 0.6, r"Times $\int x\,dx$", fontsize=12)
    fig.text(0.05, 0.2, r"\textbf{Bold} \textsl{slanted}", fontsize=12)


@usetex_case("family_monospace_courier", rc={"font.family": "monospace",
                                             "font.monospace": ["Courier"]})
def _(fig):
    fig.text(0.05, 0.6, "Courier text 0123", fontsize=12)
    fig.text(0.05, 0.2, r"\textbf{bold} $y$", fontsize=12)


@usetex_case("family_cursive", rc={"font.family": "cursive"})
def _(fig):
    fig.text(0.05, 0.5, "Zapf Chancery", fontsize=14)


@usetex_case("family_computer_modern_sans", rc={"font.family": "sans-serif"})
def _(fig):
    fig.text(0.05, 0.5, r"CM sans $\Gamma$", fontsize=14)


@usetex_case("preamble", rc={"text.latex.preamble": r"\usepackage{amssymb}\usepackage{bm}"})
def _(fig):
    fig.text(0.05, 0.6, r"$\mathbb{R}^n \ni \bm{x}$", fontsize=12)
    fig.text(0.05, 0.2, r"$\mathfrak{g} \circledcirc \lesssim$", fontsize=12)


@usetex_case("mpl_test_usetex", figsize=(6.4, 4.8))
def _(fig):
    # matplotlib's test_usetex.py::test_usetex (style mpl20).
    ax = fig.subplots()
    kwargs = {"verticalalignment": "baseline", "size": 24,
              "bbox": dict(pad=0, edgecolor="k", facecolor="none")}
    ax.text(0.2, 0.7,
            r'\LaTeX\ $\left[\int\limits_e^{-2e}'
            r'\sqrt\frac{\log^3 x}{x}\,\mathrm{d}x \right\}$',
            **kwargs)
    ax.text(0.2, 0.3, "lg", **kwargs)
    ax.text(0.4, 0.3, r"$\frac{1}{2}\pi$", **kwargs)
    ax.text(0.6, 0.3, "$p^{3^A}$", **kwargs)
    ax.text(0.8, 0.3, "$p_{3_2}$", **kwargs)
    for x in sorted({t.get_position()[0] for t in ax.texts}):
        ax.axvline(x)
    for y in sorted({t.get_position()[1] for t in ax.texts}):
        ax.axhline(y)
    ax.set_axis_off()


@usetex_case("empty", figsize=(2.0, 1.0))
def _(fig):
    # matplotlib's test_empty: a comment typesets nothing.
    fig.text(0.5, 0.5, "% a comment")


@usetex_case("unicode_minus", figsize=(2.0, 1.0))
def _(fig):
    # matplotlib's test_unicode_minus: both texts draw the same minus.
    fig.text(0.3, 0.5, "$-$")
    fig.text(0.7, 0.5, "\N{MINUS SIGN}")


@usetex_case("mathdefault", figsize=(3.2, 2.4), rc={"text.usetex": False,
                                                    "axes.formatter.use_mathtext": True})
def _(fig):
    # matplotlib's test_mathdefault: \mathdefault from the tick formatter,
    # usetex switched on before drawing.
    import matplotlib as mpl

    fig.add_subplot().set_xlim(-1, 1)
    mpl.rcParams["text.usetex"] = True


@usetex_case("eqnarray", figsize=(1.0, 1.0))
def _(fig):
    # matplotlib's test_multiline_eqnarray.
    text = (r"\begin{eqnarray*}"
            r"foo\\"
            r"bar\\"
            r"baz\\"
            r"\end{eqnarray*}")
    fig.text(0.5, 0.5, text, usetex=True, horizontalalignment="center",
             verticalalignment="center")


@usetex_case("pkg_already_loaded", rc={"text.usetex": False,
                                       "text.latex.preamble": r"\usepackage[full]{textcomp}"})
def _(fig):
    # matplotlib's test_latex_pkg_already_loaded.
    fig.text(0.5, 0.5, "hello, world", usetex=True, ha="center")


@usetex_case("underscore_preamble", rc={"text.usetex": False,
                                        "text.latex.preamble": r"\usepackage{underscore}"})
def _(fig):
    fig.text(0.5, 0.5, "hello_world", usetex=True, ha="center")


@usetex_case("single_artist", rc={"text.usetex": False})
def _(fig):
    # matplotlib's test_text.py::test_single_artist_usetex: one artist with
    # usetex among mathtext ones.
    fig.text(0.3, 0.5, r"$\frac12$", usetex=True)
    fig.text(0.6, 0.5, r"$\frac{1}{2}$")


@usetex_case("pdf_kerning", figsize=(6.4, 2.4))
def _(fig):
    # matplotlib's test_backend_pdf.py usetex texts at 50 pt (kerning and
    # ligatures in Computer Modern).
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axis_off()
    ax.text(0.05, 0.55, r"BHTem fi ffl 1234", usetex=True, fontsize=50)
    ax.text(0.05, 0.1, r"\textsf{fi ffl 1234}", usetex=True, fontsize=30)


@usetex_case("textwon", figsize=(2.0, 1.0))
def _(fig):
    # matplotlib's test_backend_svg.py::test_unicode_won.
    fig.text(0.5, 0.5, r"\textwon", usetex=True)


@usetex_case("pie_textprops", figsize=(3.2, 2.4), rc={"text.usetex": False})
def _(fig):
    # matplotlib's test_axes.py::test_pie_textprops with usetex.
    data = [23, 34, 45]
    ax = fig.subplots()
    ax.pie(data, autopct=r"%1.0f\%%", textprops={"usetex": True})
