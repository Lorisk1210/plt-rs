"""Generate the mathtext fixtures of plt-text from matplotlib.

Run with the reference venv (the tests themselves do not need Python):

    .venv/bin/python -I crates/plt-text/tests/fixtures/mathtext/generate.py

Writes `cases.txt` next to this script. The configuration is the spec's
reference one (`matplotlib.testing.setup()`, `text.hinting = "none"`) with
only matplotlib's bundled fonts installed.

Format: one block per case, tab-separated fields, floats as `repr`:

    case  <category>  <fontset>  <size>  <dpi>  <options>  <source>
    box   <width> <height> <depth>        unrounded box (Parser.parse)
    vec   <width> <height> <depth>        MathTextParser("path").parse
    ras   <width> <height> <depth>        MathTextParser("agg").parse
    g     <font file> <glyph> <code> <size> <x> <y>    VectorParse glyphs
    r     <x> <y> <width> <height>                     VectorParse rects
    err   <message>                       instead of box/vec/ras/g/r
    ext   <x0> <y0> <x1> <y1>              Text(0, 0, source).get_window_extent()
    end

Cases of category `extent` only carry `ext` (the source may hold several
lines, each typeset as math or plain text as `Text` does).

`options` is `-` or `key=value;...` with keys `default`, `fallback`, `rm`,
`it`, `bf`, `bfit`, `sf`, `tt`, `cal` (mathtext rcParams) and `family`
(the text font). In the source and messages `%`, tab and newline are
written `%25`, `%09` and `%0A`.
"""

import json
import logging
import os
import pathlib
import sys

import matplotlib

matplotlib.use("Agg")

from matplotlib import _mathtext, cbook, font_manager  # noqa: E402
from matplotlib.font_manager import FontProperties  # noqa: E402
from matplotlib.ft2font import LoadFlags  # noqa: E402
from matplotlib.mathtext import MathTextParser  # noqa: E402
from matplotlib.testing import setup  # noqa: E402

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE / "cases.txt"
SPEC_GOLDEN = HERE.parents[4] / "spec" / "goldens" / "probes" / "text_metrics.json"
SPEC_OP = "backend_agg.RendererAgg.get_text_width_height_descent"
FONTSETS = ["dejavusans", "dejavuserif", "cm", "stix", "stixsans", "custom"]

CORPUS = {
    "scripts": [
        r"$x$", r"$x^2$", r"$x_i$", r"$x_i^2$", r"$x^2_i$", r"$a^{b^{c}}$", r"$x_{i_{j}}$",
        r"$x^{2}_{3}$", r"$10^{-3}$", r"$e^{i\pi}$", r"$x^{y^{z^{w}}}$", r"$x_{a_{b_{c}}}$",
        r"$f'(x)$", r"$f''(x)$", r"$x'_i$", r"$x'^2$", r"$x^2'$", r"$'$", r"$y'''$",
        r"$x ^ 2$", r"$x_ 1$", r"$^2$", r"$_i x$", r"${}^{14}C$", r"$\alpha_1^2$",
        r"$\Gamma_{ij}^k$", r"$x^\alpha$", r"$A_{\mathrm{eff}}$", r"$f^{(n)}(x)$",
        r"$x_{max}^{min}$", r"$2^{2^{2^{2^{2^{2^{2}}}}}}$", r"$V_{x^2}$", r"$\hat{x}^2$",
        r"$(x+y)^2$", r"$\{x\}_i$", r"$x^{-}$", r"$x^+_-$", r"$e^{-x^2/2}$",
    ],
    "fractions": [
        r"$\frac{a}{b}$", r"$\frac{1}{1+\frac{1}{x}}$", r"$\dfrac{a}{b}$", r"$\frac{dy}{dx}$",
        r"$\binom{n}{k}$", r"$\genfrac{(}{)}{0}{}{a}{b}$", r"$\genfrac{[}{]}{1}{0}{a}{b}$",
        r"$\genfrac{}{}{0.5}{2}{x}{y}$", r"$\genfrac{\{}{.}{0}{3}{x}{y}$",
        r"$\frac{x^2}{y_1}$", r"$\frac{\sqrt{2}}{2}$", r"$\dfrac{\dfrac{a}{b}}{c}$",
        r"$\frac{a+b}{c-d}$", r"$\frac{1}{2}x$", r"$\frac {a} {b}$", r"$\frac{\rm a}{b} c$",
        r"$\binom{n}{k}^2$", r"$\frac{-1}{2}$", r"$x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}$",
    ],
    "radicals": [
        r"$\sqrt{x}$", r"$\sqrt[3]{x}$", r"$\sqrt{\frac{a}{b}}$", r"$\sqrt{x^2+y^2}$",
        r"$\sqrt[n]{1+x}$", r"$\sqrt{\sqrt{x}}$", r"$\sqrt{\frac{\frac{a}{b}}{c}}$",
        r"$\sqrt{2}$", r"$\sqrt[ab]{c}$", r"$\sqrt{y_1}$", r"$\sqrt{\dfrac{1}{2}}$",
    ],
    "operators": [
        r"$\sum_{i=0}^{n} i$", r"$\prod_{k=1}^{N}$", r"$\int_0^\infty$", r"$\oint$",
        r"$\int_a^b f(x)\,dx$", r"$\iint$", r"$\sum_i$", r"$\sum^n$", r"$\bigcup_{i}A_i$",
        r"$\coprod$", r"$\bigoplus_{k}$", r"$\int\int$", r"$\oint_C$", r"$\int x\,dx$",
        r"$\sum\limits$", r"$\prod_{i=1}^{\infty}\frac{1}{i}$", r"$\int_{-\infty}^{\infty}$",
    ],
    "functions": [
        r"$\sin(x) + \cos(y)$", r"$\operatorname{sinc}(x)$", r"$\lim_{x\to\infty} f(x)$",
        r"$\log_2 x$", r"$\ln x$", r"$\exp(x)$", r"$\sin^2 x$", r"$\sin^{-1} x$",
        r"$\max_{x} f$", r"$\liminf_{n} a_n$", r"$\sup$", r"$\det A$", r"$\tan\theta$",
        r"$\operatorname{arg\,max}_x$", r"$\operatorname{Tr}^2$", r"$\lim$",
        r"$\underset{x\to 0}{\lim}$", r"$\sin x\cos x$", r"$\operatorname{}$",
    ],
    "accents": [
        r"$\hat{x}\bar{y}\vec{v}\dot{a}$", r"$\tilde{n}\ddot{u}$", r"$\widehat{xyz}$",
        r"$\widetilde{abc}$", r"$\widebar{AB}$", r"$\acute{e}\grave{a}\breve{o}$",
        r"$\mathring{A}$", r"$\overrightarrow{AB}$", r"$\overleftarrow{CD}$",
        r"$\"a\'e\^o\~n\.x\`a$", r"$\dddot{x}\ddddot{y}$", r"$\hat x$", r"$\hat{\alpha}$",
        r"$\vec{F}_i$", r"$\widehat{x}$", r"$\widehat{abcdefgh}$", r"$\bar{\bar{x}}$",
        r"$\hat{}$",
    ],
    "delimiters": [
        r"$\left(\frac{a}{b}\right)$", r"$\left[\int\right]$", r"$\left\{x\right\}$",
        r"$\left|x\right|$", r"$\left(x\right)$", r"$\left\langle x\right\rangle$",
        r"$\left.\frac{a}{b}\right|_0$", r"$\left(\frac{1}{2}\middle|\frac{3}{4}\right)$",
        r"$\left\lfloor x\right\rfloor$", r"$\left\lceil \frac{x}{2}\right\rceil$",
        r"$\left(\sum_{i=0}^n\right)$", r"$\left(\right)$", r"$\left[\left(x\right)\right]$",
        r"$\left\Vert x\right\Vert$", r"$\left/x\right\backslash$", r"$\left\uparrow x\right\downarrow$",
        r"$\left(\dfrac{\dfrac{a}{b}}{c}\right)$", r"$\left< x \right>$",
        r"$\left\{\frac{a}{b}\middle\|\frac{c}{d}\right\}$", r"$\left( a b \right)$",
        r"$\left(\rm AV\right)$", r"$\left( 1 2 \right)$", r"$\left(\middle.\right)$",
        r"$( [ \{ \} ] )$", r"$\lbrace x \rbrace$", r"$\langle x \rangle$",
    ],
    "fonts": [
        r"$\mathrm{d}x$", r"$\mathbf{F}$", r"$\mathit{x}$", r"$\mathcal{L}$", r"$\mathbb{R}$",
        r"$\mathsf{A}$", r"$\mathtt{x}$", r"$\mathfrak{g}$", r"$\mathscr{L}$",
        r"$\mathbfit{x}$", r"$\mathrm{Re}\{z\}$", r"$\mathbb{NZQRC}$", r"$\mathbb{abc123}$",
        r"$\mathcal{ABCXYZ}$", r"$\mathfrak{AbCd}$", r"$\mathsf{Sans\ 123}$",
        r"$\mathtt{mono}$", r"$\mathdefault{default}$", r"$\mathregular{x^2}$",
        r"$\mathnormal{x2}$", r"$\rm roman \it italic \bf bold$", r"$\boldsymbol{\alpha x}$",
        r"$\boldsymbol{a+b}$", r"$\boldsymbol{\Gamma 1}$", r"$\text{for all}\ x$",
        r"$\text{a}\text{b}$", r"$\mathbf{\alpha\Omega}$", r"$\mathsf{\alpha+\beta}$",
        r"$\mathbb{\Gamma\pi}$", r"$\mathit{\Gamma\alpha}$", r"$\mathrm{\alpha}$",
        r"$\mathtt{\{x\}}$", r"$\mathbf{\partial}$", r"$\mathsf{\partial\nabla}$",
        r"$\cal L$", r"$\mathcal{\alpha}$", r"$\mathfrak{\alpha}$", r"$\mathscr{\alpha}$",
        r"$\mathbfit{\alpha}$", r"$\mathbb{\alpha}$", r"$\mathsf{\Gamma}$",
    ],
    "spacing": [
        r"$a\,b\;c\quad d\qquad e$", r"$a\!b$", r"$a\:b\>c\ d~e$", r"$a\enspace b$",
        r"$a\thinspace b$", r"$a\/b$", r"$a\hspace{1.5}b$", r"$a\hspace{-0.5}b$",
        r"$a \hspace {.5} b$", r"$a\quad$", r"$\,$", r"$\quad\qquad$", r"$a\hspace{2.}b$",
    ],
    "symbols": [
        r"$\alpha\beta\Gamma$", r"$\infty \pm \times \div$", r"$\leq \geq \neq \approx$",
        r"$\rightarrow \Leftarrow$", r"$2\pi r$", r"$100\%$", r"$\$5$", r"$\not=$",
        r"$\ldots \cdots$", r"$\vdots\ddots$", r"$-1$", "$−1$", r"$a-b$", r"$a=-b$",
        r"$(-x)$", r"$x^{-1}$", r"${-1}$", r"$ -2$", r"$=-2$", r"$1.5$", r"$1,000$",
        r"${,}$", r"$a, b$", r"$a; b$", r"$x!$", r"$a.b$", r"$1. 5$", r"$a+b=c$",
        r"$a \cdot b$", r"$a \ast b$", r"$a \star b$", r"$\partial \nabla \hbar \ell$",
        r"$\aleph \forall \exists \emptyset$", r"$\Re \Im \wp$", r"$\prime$",
        r"$\alpha\beta\gamma\delta\epsilon\zeta\eta\theta\iota\kappa\lambda\mu$",
        r"$\nu\xi\pi\rho\sigma\tau\upsilon\phi\chi\psi\omega$",
        r"$\Gamma\Delta\Theta\Lambda\Xi\Pi\Sigma\Upsilon\Phi\Psi\Omega$",
        r"$\varepsilon\vartheta\varpi\varrho\varsigma\varphi$", r"$\to \mapsto \Rightarrow$",
        r"$\subset\supset\in\notin\cup\cap$", r"$\langle\rangle\lfloor\rfloor$",
        r"$\{ \} \| | /$", r"$\#\_\&$", r"$\$$", r"$\backslash$", r"$\circ\bullet$",
        r"$\oplus\otimes\odot$", r"$\sim\simeq\cong\equiv$", r"$\leftarrow\uparrow$",
        r"$\clubsuit\diamondsuit\heartsuit\spadesuit$", r"$\triangle\square$",
        r"$\[$", r"$\]$", r"$a\|b$", r"$\vert x\vert$", r"$\cdotp\ldotp$",
        r"$\__sqrt__$", r"$\#a$", r"$A V T o$", r"$AVATAR$", r"$\pm 1$", r"$1 \pm$",
        r"$\sum$", r"$\dotplus\dotminus$", r"$\Cap\Cup$", r"$\varnothing$",
    ],
    "unicode": [
        "$αé$", "$αβγ$", "$∑∫∏$", "$→≤≥$", "$x²$", "$ℝℂ$", "$中$", "$😀$", "$€£¥$",
        "$ÅÆØ$", "$ΓΔ$", "$x×y$", "$a−b$", "$∂f$", "$ℏ$",
    ],
    "text": [
        "foo $x$ bar", r"$a$ and $b$", "$x$", r"price \$5 is $x$", "text only",
        "$x$ a $x$ b", r"a $\alpha$ b $\beta$ c", "AV $T$ ok", "$x$  ", "  $x$",
        "", " ", "$a$$b$", r"\$ $x$ \$", "mixed $x^2$ end",
    ],
    "misc": [
        r"$\overset{a}{=}$", r"$\underset{x}{=}$", r"$\overset{\mathrm{def}}{=}$",
        r"$\substack{a\\b}$", r"$\substack{i<n \\ j<m}$", r"$\sum_{\substack{0<i<m\\0<j<n}}$",
        r"$\phantom{x}y$", r"$\llap{a}b$", r"$\rlap{a}b$", r"$\overline{abc}$",
        r"$\underline{x}$", r"$\overline{\overline{x}}$", r"$\underline{\frac{a}{b}}$",
        r"$\phantom{x}^2$", r"${}$", r"$\rm$", r"$\overline{x}^2$", r"$\underline{y}_1$",
        r"$\sqrt{x}^2$", r"$\frac{a}{b}^2$", r"$\left(x\right)^2$", r"$\left(x\right)_i^2$",
        r"$\mathrm{x}^{\mathrm{y}}$", r"$x\phantom{y}z$", r"$\overset{!}{=}$",
    ],
    "errors": [
        r"$x^$", r"$\frac{a}$", r"$\foo$", r"$x^2^3$", r"$\left(x$", r"$\sqrt[$", r"${$",
        r"$\sqrt{}$", r"$\frac{}{}$", "$$", r"$\stackrel{a}{=}$", r"$x=\mathrm{%}$",
        r"$a+x^2^3$", r"$a+x_2_3$", r"$x^{a$", r"$\hat$", r"$\hat}$", r"$x^}$",
        r"$\genfrac{(}{)}{0}{}{a}$", r"$\overset{a}$", r"$\text$", r"$\substack$",
        r"$\left(x\right$", r"$\left x\right)$", r"$\left(x\middle\right)$",
        r"$\operatorname$", r"$\boldsymbol$", r"$\sqrt[3$", r"$a$ and $\foo$",
        r"$a$ and $a#$", "$a\nb$", r"$x^{\hat}$", r"$\phantom$", r"$\llap{a$", r"$}$",
        r"$a}$", r"$\frac{a}{b$", r"$\mathrm{a$", r"$ $", r"$\binom{a}$", r"$\dfrac$",
        r"$\underline$", r"$\overline{$", r"$\textbf{x}$", r"$\tfrac{a}{b}$",
        r"$\mathcircled{a}$", r"$\hspace{x}$", r"$\left($", r"$a$ $", r"$\substack{}$",
        r"$\left(\middle|\right)$", r"$\substack{{a}}$", r"$\fracx$", r"$x__2$",
        r"$\left\foo x\right)$", r"$\sqrt[]{x}$", r"$\rightx$",
    ],
}

# Text extents (default alignment, anchored at the origin).
EXTENT = [
    r"$x^2$", r"$\frac{a}{b}$", r"$\sum_{i=0}^{n} i$", "foo $x$ bar", r"$\sqrt{x}$",
    r"$\left(\frac{a}{b}\right)$", "$x^2$\n$\\frac{a}{b}$\ny", r"$\int_0^\infty$",
    "a\n$b_i$", r"$\alpha$ and $\beta$", "$x$\n\n$y$",
]

# Representative cases at other sizes and resolutions.
SIZED = [
    r"$x^2$", r"$\frac{a}{b}$", r"$\sum_{i=0}^{n} i$", "foo $x$ bar", r"$\sqrt{x}$",
    r"$\left(\frac{a}{b}\right)$", r"$\int_0^\infty$", r"$\widehat{xyz}$", r"$a\,b\quad c$",
]
SIZES = [(6, 100), (20, 100), (48, 100), (10, 72), (10, 150), (12, 300), (9.5, 96)]

# Custom fontset configurations.
CUSTOM = [
    "rm=serif;it=serif:italic;bf=serif:bold",
    "rm=cmr10;it=cmmi10;bf=cmb10;fallback=cm",
    "rm=STIXGeneral;it=STIXGeneral:italic;fallback=stix",
    "fallback=stixsans",
    "fallback=none",
    "default=it",
    "default=rm",
    "family=DejaVu Serif",
]
CUSTOM_CORPUS = [
    r"$x^2$", r"$\frac{a}{b}$", r"$\sum_{i=0}^{n} i$", r"$\left(\frac{a}{b}\right)$",
    r"$\alpha\beta$", r"$a \times b - c$", r"$\mathcal{L}$", r"$\mathbb{R}$",
    r"$\sqrt{x}$", r"$\int_0^1$", "foo $x$ bar", r"$\clubsuit$", r"$\prime$", r"$f'$",
]
OTHER_FONTSET_OPTIONS = ["default=it", "default=rm", "default=bb", "family=DejaVu Serif"]


def bundled_fonts_only():
    fm = font_manager.fontManager
    logging.getLogger("matplotlib.font_manager").setLevel(logging.ERROR)
    logging.getLogger("matplotlib.mathtext").setLevel(logging.ERROR)
    fm.ttflist, fm.afmlist = [], []
    dirs = [cbook._get_data_path("fonts", d) for d in ("ttf", "afm", "pdfcorefonts")]
    for fontext in ("afm", "ttf"):
        paths = font_manager.findSystemFonts(dirs, fontext=fontext)
        for path in sorted(paths, key=os.path.basename):
            fm.addfont(path)
    fm._findfont_cached.cache_clear()
    font_manager._get_font.cache_clear()


def encode(text):
    return text.replace("%", "%25").replace("\t", "%09").replace("\n", "%0A")


def rc_for(fontset, options):
    rc = {"mathtext.fontset": fontset}
    family = None
    if options != "-":
        for item in options.split(";"):
            key, value = item.split("=", 1)
            if key == "family":
                family = value
            else:
                rc[f"mathtext.{key}"] = value
    return rc, family


def spec_cases():
    """The mathtext probes of the spec (`get_text_width_height_descent` with
    `ismath=True`) as `(fontset, size, dpi, source, golden value)`."""
    probes = json.loads(SPEC_GOLDEN.read_text())["probes"]
    for probe in probes:
        args = probe["args"]
        if probe["op"] != SPEC_OP or not args.get("ismath"):
            continue
        fontset = (args.get("rc") or {}).get("mathtext.fontset", "dejavusans")
        size = (args.get("font") or {}).get("size", 10)
        yield fontset, size, args.get("dpi", 100), args["s"], probe.get("value")


def run_case(category, fontset, size, dpi, options, source):
    rc, family = rc_for(fontset, options)
    lines = ["\t".join(["case", category, fontset, repr(float(size)), repr(float(dpi)),
                        options, encode(source)])]
    with matplotlib.rc_context(rc):
        MathTextParser._parse_cached.cache_clear()
        prop = FontProperties(family=family, size=size)
        try:
            fontset_obj = MathTextParser._font_type_mapping[fontset](
                prop, LoadFlags.NO_HINTING)
            box = _mathtext.Parser().parse(source, fontset_obj, size, dpi)
            vec = MathTextParser("path").parse(source, dpi=dpi, prop=prop)
            ras = MathTextParser("agg").parse(source, dpi=dpi, prop=prop)
        except Exception as err:  # noqa: BLE001 -- every failure is recorded
            lines.append(f"err\t{encode(str(err))}")
            lines.append("end")
            return lines
    lines.append(f"box\t{box.width!r}\t{box.height!r}\t{box.depth!r}")
    lines.append(f"vec\t{float(vec.width)!r}\t{float(vec.height)!r}\t{float(vec.depth)!r}")
    lines.append(f"ras\t{float(ras.width)!r}\t{float(ras.height)!r}\t{float(ras.depth)!r}")
    for font, fontsize, code, glyph, x, y in vec.glyphs:
        name = os.path.basename(font.fname)
        lines.append(f"g\t{name}\t{int(glyph)}\t{int(code)}\t{float(fontsize)!r}"
                     f"\t{float(x)!r}\t{float(y)!r}")
    for x, y, w, h in vec.rects:
        lines.append(f"r\t{float(x)!r}\t{float(y)!r}\t{float(w)!r}\t{float(h)!r}")
    lines.append("end")
    return lines


def extent_case(fontset, source):
    from matplotlib.backends.backend_agg import FigureCanvasAgg
    from matplotlib.figure import Figure

    with matplotlib.rc_context({"mathtext.fontset": fontset}):
        MathTextParser._parse_cached.cache_clear()
        fig = Figure(figsize=(6.4, 4.8), dpi=100)
        renderer = FigureCanvasAgg(fig).get_renderer()
        text = fig.text(0, 0, source)
        (x0, y0), (x1, y1) = text.get_window_extent(renderer).get_points()
    return [
        "\t".join(["case", "extent", fontset, "10.0", "100.0", "-", encode(source)]),
        f"ext\t{float(x0)!r}\t{float(y0)!r}\t{float(x1)!r}\t{float(y1)!r}",
        "end",
    ]


def main():
    setup()
    matplotlib.rcParams["text.hinting"] = "none"
    bundled_fonts_only()
    out = []
    for fontset in FONTSETS:
        for category, sources in CORPUS.items():
            for source in sources:
                out.extend(run_case(category, fontset, 10, 100, "-", source))
        for size, dpi in SIZES:
            for source in SIZED:
                out.extend(run_case("sizes", fontset, size, dpi, "-", source))
        for source in EXTENT:
            out.extend(extent_case(fontset, source))
        options = CUSTOM if fontset == "custom" else OTHER_FONTSET_OPTIONS
        for option in options:
            for source in CUSTOM_CORPUS:
                out.extend(run_case("options", fontset, 10, 100, option, source))
    for fontset, size, dpi, source, golden in spec_cases():
        lines = run_case("spec", fontset, size, dpi, "-", source)
        if golden is None:
            assert any(line.startswith("err\t") for line in lines), source
        else:
            vec = next(line for line in lines if line.startswith("vec\t"))
            assert [float(v) for v in vec.split("\t")[1:]] == golden, (source, golden)
        out.extend(lines)
    OUT.write_text("\n".join(out) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
