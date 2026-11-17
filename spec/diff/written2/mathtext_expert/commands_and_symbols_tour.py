# A tour of mathtext commands (accents, spacing, operatorname, overset, genfrac, substack, boldsymbol, dots), extents per command.
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

exprs = [
    r"$\hat{a}\ \tilde{b}\ \bar{c}\ \vec{d}\ \dot{e}\ \ddot{f}\ \breve{g}\ \acute{h}\ \grave{i}$",
    r"$a\,b\;c\quad d\qquad e\!f\ g$",
    r"$\operatorname{sinc}(x) = \operatorname{sin}(x)/x$",
    r"$\overset{\mathrm{def}}{=}\ \underset{n\to\infty}{\lim}$",
    r"$\genfrac{[}{]}{0}{}{n}{k} \dfrac{1}{2} \binom{n}{k}$",
    r"$\sum_{\substack{0<i<m \\ 0<j<n}} P(i,j)$",
    r"$\boldsymbol{\alpha x} \mathbf{\alpha x}$",
    r"$\ldots \cdots \vdots \ddots$",
    r"$\left\{ \frac{a}{b} \right\rangle \left| x \right|$",
    r"$\widehat{xyz}\ \widetilde{abc}\ \overline{AB}\ \underline{CD}$",
    r"$\mathbb{R}\ \mathfrak{g}\ \mathscr{L}$",
    r"$\sqrt{\sqrt{\sqrt{x}}}\ x^{y^{z^{w}}}$",
]
fig = Figure(figsize=(6, 7), dpi=80)
FigureCanvasAgg(fig)
texts = [fig.text(0.02, 0.95 - k * 0.075, e, fontsize=12) for k, e in enumerate(exprs)]
fig.canvas.draw()
r = fig.canvas.get_renderer()
for t in texts:
    b = t.get_window_extent(r)
    print(round(b.width, 2), round(b.height, 2))
fig.savefig("tour.png")
for fs in ("cm", "stix"):
    fig.texts.clear()
    with matplotlib.rc_context({"mathtext.fontset": fs}):
        texts = [fig.text(0.02, 0.95 - k * 0.075, e, fontsize=12) for k, e in enumerate(exprs)]
        fig.canvas.draw()
        print(fs, [round(t.get_window_extent(r).width, 1) for t in texts])
        fig.savefig(f"tour_{fs}.png")
