r"""Mathtext rendered in every fontset: expressions from matplotlib's own
``tests/test_mathtext.py`` drawn the way its image tests draw them
(``fig.text(0.5, y, s, fontsize=12, ha="center", va="center")``), one row of
0.75 in per expression on a 5.25 in wide figure.

* ``tests_a_<fontset>``, ``tests_b_<fontset>``: twenty ``math_tests``.
* ``fonts_<fontset>``: ten ``font_tests`` (every font command).
* ``delimiters_<fontset>``: the ``svgastext_math_tests`` (every
  AutoHeightChar substitution at five sizes) for Computer Modern, DejaVu
  Sans and STIX.
* ``lightweight``: the ``lightweight_math_tests`` with
  ``math_fontfamily="dejavusans"`` on the texts while ``mathtext.fontset``
  is ``cm`` (``test_mathtext_rendering_lightweight``).
"""

from spec_support import case

FONTSETS = ["dejavusans", "dejavuserif", "cm", "stix", "stixsans", "custom"]
WIDTH = 5.25
ROW = 0.75

TESTS_A = [
    r'$a+b+\dot s+\dot{s}+\ldots$',
    r'$x\ x\,x\;x\quad x\qquad x\!x\hspace{ 0.5 }y$',
    r'$\left[\left\lfloor\frac{5}{\frac{\left(3\right)}{4}} y\right)\right]$',
    (r'$\sum _{\genfrac{}{}{0}{}{0\leq i\leq m}{0<j<n}}f\left(i,j\right)'
     r'\mathcal{R}\prod_{i=\alpha_{i+1}}^\infty a_i \sin(2 \pi f x_i)'
     r"\sqrt[2]{\prod^\frac{x}{2\pi^2}_\infty}$"),
    r'$x = \frac{x+\frac{5}{2}}{\frac{y+3}{8}}$',
    r'Foo: $\alpha_{i+1}^j = {\rm sin}(2\pi f_j t_i) e^{-5 t_i/\tau}$',
    r'$\ddot{o}\acute{e}\grave{e}\hat{O}\breve{\imath}\tilde{n}\vec{q}$',
    r'$\limsup_{x\to\infty}$',
    r"$\sqrt[3]{\frac{X_2}{Y}}=5$",
    r'$\mathcal{H} = \int d \tau \left(\epsilon E^2 + \mu H^2\right)$',
]

TESTS_B = [
    r'$\widehat{abc}\widetilde{def}$',
    r'${a}_{0}+\frac{1}{{a}_{1}+\frac{1}{{a}_{2}+\frac{1}{{a}_{3}+\frac{1}{{a}_{4}}}}}$',
    r'$\binom{p}{2}{x}^{2}{y}^{p-2}-\frac{1}{1-x}\frac{1}{1-{x}^{2}}$',
    r'$\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+x}}}}}}}$',
    r'$\left(\frac{{\partial }^{2}}{\partial {x}^{2}}+\frac{{\partial }^{2}}{\partial {y}^{2}}\right){|\varphi \left(x+iy\right)|}^{2}=0$',
    r'${x}_{{y}_{b}^{a}}^{{z}_{c}^{d}}$',
    r"$\left\Vert \frac{a}{b} \right\Vert \left\vert \frac{a}{b} \right\vert \left\| \frac{a}{b}\right\| \left| \frac{a}{b} \right| \Vert a \Vert \vert b \vert \| a \| | b |$",
    r'$M \, M \thinspace M \/ M \> M \: M \; M \ M \enspace M \quad M \qquad M \! M$',
    r'$\Cap$ $\Cup$ $\leftharpoonup$ $\barwedge$ $\rightharpoonup$',
    r'$6-2$; $-2$; $ -2$; ${-2}$; ${  -2}$; $20^{+3}_{-2}$',
]

FONTS = [
    r'mathrm $\mathrm{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$',
    r'mathbf $\mathbf{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$',
    r'mathit $\mathit{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$',
    r'mathtt $\mathtt{abcdefghijklmnopqrstuvwxyz}$',
    r'mathbb $\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$',
    r'mathrm mathbb $\mathrm{\mathbb{\Gamma \Pi \Sigma \gamma \pi}}$',
    r'mathcal $\mathcal{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$',
    r'mathfrak $\mathfrak{abcdefghijklmnopqrstuvwxyz}$',
    r'mathscr $\mathscr{abcdefghijklmnopqrstuvwxyz}$',
    r'mathbfit $\mathbfit{0123456789}$ mathsf $\mathsf{0123456789}$',
]

DELIMITERS = [
    r'$-$-',
    *[
        r'$\left' + lc + r' M \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{M}{B} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{\frac{M}{I}}{B} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{\frac{M}{I}}{\frac{B}{U}} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r'\frac{\frac{\frac{M}{I}}{N}}{\frac{\frac{B}{U}}{G}} \middle/ ? \middle\backslash ? \right' + rc + '$'
        for lc, rc in ['()', '[]', '<>', (r'\{', r'\}'), (r'\lfloor', r'\rfloor'), (r'\lceil', r'\rceil')]
    ],
]

LIGHTWEIGHT = [
    r'$\sqrt[ab]{123}$',
    r'$x \overset{f}{\rightarrow} \overset{f}{x} \underset{xx}{ff} \overset{xx}{ff} \underset{f}{x} \underset{f}{\leftarrow} x$',
    r'$\sum x\quad\sum^nx\quad\sum_nx\quad\sum_n^nx\quad\prod x\quad\prod^nx\quad\prod_nx\quad\prod_n^nx$',
    r'$1.$ $2.$ $19680801.$ $a.$ $b.$ $mpl.$',
    r'$\text{text}_{\text{sub}}^{\text{sup}} + \text{\$foo\$} + \frac{\text{num}}{\mathbf{\text{den}}}\text{with space, curly brackets \{\}, and dash -}$',
    r'$\boldsymbol{abcde} \boldsymbol{+} \boldsymbol{\Gamma + \Omega} \boldsymbol{01234} \boldsymbol{\alpha * \beta}$',
    r'$\left\lbrace\frac{\left\lbrack A^b_c\right\rbrace}{\left\leftbrace D^e_f \right\rbrack}\right\rightbrace\ \left\leftparen\max_{x} \left\lgroup \frac{A}{B}\right\rgroup \right\rightparen$',
    r'$\left( a\middle. b \right)$ $\left( \frac{a}{b} \middle\vert x_i \in P^S \right)$ $\left[ 1 - \middle| a\middle| + \left( x  - \left\lfloor \dfrac{a}{b}\right\rfloor \right)  \right]$',
    r'$\sum_{\substack{k = 1\\ k \neq \lfloor n/2\rfloor}}^{n}P(i,j) \sum_{\substack{i \neq 0\\ -1 \leq i \leq 3\\ 1 \leq j \leq 5}} F^i(x,y) \sum_{\substack{\left \lfloor \frac{n}{2} \right\rfloor}} F(n)$',
    ' '.join(f'${c}\\underline{{{c}}}$' for c in 'abfghy') + r' $\underline{\left(\dfrac{num}{\underline{den}}\right)}^{\underline{p_1}}$',
]


def _rows(expressions, **kw):
    """One centred expression per row, top to bottom."""
    def build(fig):
        n = len(expressions)
        for i, s in enumerate(expressions):
            fig.text(0.5, 1 - (i + 0.5) / n, s, fontsize=12, ha="center", va="center", **kw)

    return build


for _fontset in FONTSETS:
    _rc = {"mathtext.fontset": _fontset}
    for _name, _expressions in [("tests_a", TESTS_A), ("tests_b", TESTS_B), ("fonts", FONTS)]:
        case(f"mathtext/{_name}_{_fontset}", figsize=(WIDTH, ROW * len(_expressions)),
             rc=_rc)(_rows(_expressions))

for _fontset in ["cm", "dejavusans", "stix"]:
    case(f"mathtext/delimiters_{_fontset}", figsize=(WIDTH, ROW * len(DELIMITERS)),
         rc={"mathtext.fontset": _fontset})(_rows(DELIMITERS))

case("mathtext/lightweight", figsize=(WIDTH, ROW * len(LIGHTWEIGHT)),
     rc={"mathtext.fontset": "cm"})(_rows(LIGHTWEIGHT, math_fontfamily="dejavusans"))
