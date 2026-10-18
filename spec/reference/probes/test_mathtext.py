r"""Mathtext typesetting on matplotlib's own test expressions, in every fontset.

Area ``mathtext`` (golden ``spec/goldens/probes/mathtext.json``).

Environment
===========
* Reference rcParams: ``matplotlib.testing.setup()`` plus
  ``text.hinting = 'none'`` (so ``font.family = 'DejaVu Sans'``,
  ``font.size = 10``, ``mathtext.fallback = 'cm'``, ``mathtext.default =
  'it'`` and the default ``mathtext.rm/it/bf/bfit/sf/tt/cal`` of the
  ``custom`` fontset: ``sans``, ``sans:italic``, ``sans:bold``,
  ``sans:italic:bold``, ``sans``, ``monospace``, ``cursive``).
* Only matplotlib's bundled fonts are installed (``spec_support.fonts``;
  the same database as the ``text_metrics`` area).

Ops
===
``mathtext.MathTextParser.parse`` -- args ``{s, fontset, size, dpi}``.
Under ``rc_context({"mathtext.fontset": fontset})``, ``prop =
FontProperties(size=size)`` and ``vec = MathTextParser("path").parse(s,
dpi=dpi, prop=prop)``; ``box`` is the root box of
``_mathtext.Parser().parse(s, fonts, size, dpi)`` with ``fonts`` the fontset
class ``MathTextParser`` uses, built with ``LoadFlags.NO_HINTING`` (exactly
what ``MathTextParser("path")`` ships).  Returns::

    {"box":    [width, height, depth],      # unrounded, pixels
     "extent": [width, height, depth],      # VectorParse: ceil(w),
                                            # ceil(h) + ceil(d), ceil(d)
     "fonts":  [basename, ...],             # font files, in order of first use
     "glyphs": [[font, size, code, x, y], ...],
     "rects":  [[x, y, width, height], ...]}

``glyphs`` and ``rects`` are ``VectorParse.glyphs``/``.rects`` in the order
``ship`` emits them (depth-first through the box tree): ``font`` indexes
``fonts``, ``size`` is the font size in points, ``code`` the character code
looked up in that font (for Computer Modern the BaKoMa code, e.g. a sized
delimiter of ``cmex10``), ``x``/``y`` the glyph origin in pixels, ``y``
upwards from the baseline of the rounded box, i.e. the line ``ceil(h)``
below its top (``ceil(h) - y_ship + offset``).  Rects are fraction bars,
radical overbars and ``\overline``/``\underline`` rules, ``y`` their bottom
edge in the same frame.  Invalid mathtext records the exception type:
``ValueError``, or ``IndexError`` where matplotlib's own parse action
crashes.

Expressions
===========
``math_tests``, ``svgastext_math_tests``, ``lightweight_math_tests`` and
``font_tests`` of ``matplotlib/tests/test_mathtext.py`` (3.11.2; the
``None`` placeholders dropped), the expressions of that module's
figure-equality tests, the symbols of ``test_mathtext_operators`` and the
invalid inputs of ``test_mathtext_exceptions``.  The lists are copied here
because ``matplotlib.tests`` cannot be imported without its baseline images.
Every expression is typeset with every fontset at 12 pt / 100 dpi (the size
matplotlib's image tests use) and 10 pt / 72 dpi (``parse``'s default dpi).

A systematic sweep built from the parser's own tables adds, in every fontset
at 12 pt / 100 dpi: every named symbol of ``_mathtext_data.tex2uni`` that
parses on its own (in groups of twelve), every accent and wide accent, every
font name both as ``\mathNAME{...}`` and as a ``\NAME`` switch, every
function name and over/under or integral symbol with and without limits,
every delimiter after ``\left`` at five heights, every spacing command, and
fractions, radicals, stacking, scripts, operator names, text mode, escapes,
kerning and non-ASCII input.
"""

from __future__ import annotations

import os

import matplotlib
import pytest
from matplotlib import _mathtext
from matplotlib.font_manager import FontProperties
from matplotlib.ft2font import LoadFlags
from matplotlib.mathtext import MathTextParser

from spec_support import ProbeSet
from spec_support.fonts import bundled_fonts_only

probes = ProbeSet("mathtext")

# Missing-glyph and findfont fallback chatter is expected (the custom
# fontset asks for a cursive font, which is not bundled).
pytestmark = pytest.mark.filterwarnings("ignore::UserWarning")

FONTSETS = ["dejavusans", "dejavuserif", "cm", "stix", "stixsans", "custom"]
SIZES = [(12.0, 100.0), (10.0, 72.0)]

# --------------------------------------------------------------------------
# matplotlib/tests/test_mathtext.py (3.11.2), placeholders dropped
# --------------------------------------------------------------------------

MATH_TESTS = [
    r'$a+b+\dot s+\dot{s}+\ldots$',
    r'$x\hspace{-0.2}\doteq\hspace{-0.2}y$',
    r'\$100.00 $\alpha \_$',
    r'$\frac{\$100.00}{y}$',
    r'$x   y$',
    r'$x+y\ x=y\ x<y\ x:y\ x,y\ x@y$',
    r'$100\%y\ x*y\ x/y x\$y$',
    r'$x\leftarrow y\ x\forall y\ x-y$',
    r'$x \sf x \bf x {\cal X} \rm x$',
    r'$x\ x\,x\;x\quad x\qquad x\!x\hspace{ 0.5 }y$',
    r'$\{ \rm braces \}$',
    r'$\left[\left\lfloor\frac{5}{\frac{\left(3\right)}{4}} y\right)\right]$',
    r'$\left(x\right)$',
    r'$\sin(x)$',
    r'$x_2$',
    r'$x^2$',
    r'$x^2_y$',
    r'$x_y^2$',
    (r'$\sum _{\genfrac{}{}{0}{}{0\leq i\leq m}{0<j<n}}f\left(i,j\right)'
     r'\mathcal{R}\prod_{i=\alpha_{i+1}}^\infty a_i \sin(2 \pi f x_i)'
     r"\sqrt[2]{\prod^\frac{x}{2\pi^2}_\infty}$"),
    r'$x = \frac{x+\frac{5}{2}}{\frac{y+3}{8}}$',
    r'$dz/dt = \gamma x^2 + {\rm sin}(2\pi y+\phi)$',
    r'Foo: $\alpha_{i+1}^j = {\rm sin}(2\pi f_j t_i) e^{-5 t_i/\tau}$',
    r'Variable $i$ is good',
    r'$\Delta_i^j$',
    r'$\Delta^j_{i+1}$',
    r'$\ddot{o}\acute{e}\grave{e}\hat{O}\breve{\imath}\tilde{n}\vec{q}$',
    r"$\arccos((x^i))$",
    r"$\gamma = \frac{x=\frac{6}{8}}{y} \delta$",
    r'$\limsup_{x\to\infty}$',
    r"$f'\quad f'''(x)\quad ''/\mathrm{yr}$",
    r'$\frac{x_2888}{y}$',
    r"$\sqrt[3]{\frac{X_2}{Y}}=5$",
    r"$\sqrt[3]{x}=5$",
    r'$\frac{X}{\frac{X}{Y}}$',
    r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2} \int^{\alpha_2}_{\alpha_2} d \alpha^\prime_2 \left[\frac{ U^{2\beta}_{\delta_1 \rho_1} - \alpha^\prime_2U^{1\beta}_{\rho_1 \sigma_2} }{U^{0\beta}_{\rho_1 \sigma_2}}\right]$",
    r'$\mathcal{H} = \int d \tau \left(\epsilon E^2 + \mu H^2\right)$',
    r'$\widehat{abc}\widetilde{def}$',
    '$\\Gamma \\Delta \\Theta \\Lambda \\Xi \\Pi \\Sigma \\Upsilon \\Phi \\Psi \\Omega$',
    '$\\alpha \\beta \\gamma \\delta \\epsilon \\zeta \\eta \\theta \\iota \\lambda \\mu \\nu \\xi \\pi \\kappa \\rho \\sigma \\tau \\upsilon \\phi \\chi \\psi$',
    r'${x}^{2}{y}^{2}$',
    r'${}_{2}F_{3}$',
    r'$\frac{x+{y}^{2}}{k+1}$',
    r'$x+{y}^{\frac{2}{k+1}}$',
    r'$\frac{a}{b/2}$',
    r'${a}_{0}+\frac{1}{{a}_{1}+\frac{1}{{a}_{2}+\frac{1}{{a}_{3}+\frac{1}{{a}_{4}}}}}$',
    r'${a}_{0}+\dfrac{1}{{a}_{1}+\dfrac{1}{{a}_{2}+\dfrac{1}{{a}_{3}+\dfrac{1}{{a}_{4}}}}}$',
    r'$\binom{n}{k/2}$',
    r'$\binom{p}{2}{x}^{2}{y}^{p-2}-\frac{1}{1-x}\frac{1}{1-{x}^{2}}$',
    r'${x}^{2y}$',
    r'$\sum _{i=1}^{p}\sum _{j=1}^{q}\sum _{k=1}^{r}{a}_{ij}{b}_{jk}{c}_{ki}$',
    r'$\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+x}}}}}}}$',
    r'$\left(\frac{{\partial }^{2}}{\partial {x}^{2}}+\frac{{\partial }^{2}}{\partial {y}^{2}}\right){|\varphi \left(x+iy\right)|}^{2}=0$',
    r'${2}^{{2}^{{2}^{x}}}$',
    r'${\int }_{1}^{x}\frac{\mathrm{dt}}{t}$',
    r'$\int {\int }_{D}\mathrm{dx} \mathrm{dy}$',
    r'${y}_{{x}^{2}}$',
    r'${y}_{{x}_{2}}$',
    r'${x}_{92}^{31415}+\pi $',
    r'${x}_{{y}_{b}^{a}}^{{z}_{c}^{d}}$',
    r'${y}_{3}^{\prime \prime \prime }$',
    r"$\left( \xi \left( 1 - \xi \right) \right)$",
    r"$\left(2 \, a=b\right)$",
    r"$? ! &$",
    r"$\left\Vert \frac{a}{b} \right\Vert \left\vert \frac{a}{b} \right\vert \left\| \frac{a}{b}\right\| \left| \frac{a}{b} \right| \Vert a \Vert \vert b \vert \| a \| | b |$",
    r'$\mathring{A}  \AA$',
    r'$M \, M \thinspace M \/ M \> M \: M \; M \ M \enspace M \quad M \qquad M \! M$',
    r'$\Cap$ $\Cup$ $\leftharpoonup$ $\barwedge$ $\rightharpoonup$',
    r'$\hspace{-0.2}\dotplus\hspace{-0.2}$ $\hspace{-0.2}\doteq\hspace{-0.2}$ $\hspace{-0.2}\doteqdot\hspace{-0.2}$ $\ddots$',
    r'$xyz^kx_kx^py^{p-2} d_i^jb_jc_kd x^j_i E^0 E^0_u$',
    r'${xyz}^k{x}_{k}{x}^{p}{y}^{p-2} {d}_{i}^{j}{b}_{j}{c}_{k}{d} {x}^{j}_{i}{E}^{0}{E}^0_u$',
    r'${\int}_x^x x\oint_x^x x\int_{X}^{X}x\int_x x \int^x x \int_{x} x\int^{x}{\int}_{x} x{\int}^{x}_{x}x$',
    r'testing$^{123}$',
    r'$6-2$; $-2$; $ -2$; ${-2}$; ${  -2}$; $20^{+3}_{-2}$',
    r'$\overline{\omega}^x \frac{1}{2}_0^x$',
    r'$,$ $.$ $1{,}234{, }567{ , }890$ and $1,234,567,890$',
    r'$\left(X\right)_{a}^{b}$',
    r'$\dfrac{\$100.00}{y}$',
    r'$a=-b-c$',
]

SVGASTEXT_MATH_TESTS = [
    r'$-$-',
    # Every AutoHeightChar substitution, at five delimiter sizes.
    *[
        r'$\left' + lc + r' M \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{M}{B} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{\frac{M}{I}}{B} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r' \frac{\frac{M}{I}}{\frac{B}{U}} \middle/ ? \middle\backslash ? \right' + rc + ' ' +
        r'\left' + lc + r'\frac{\frac{\frac{M}{I}}{N}}{\frac{\frac{B}{U}}{G}} \middle/ ? \middle\backslash ? \right' + rc + '$'
        for lc, rc in ['()', '[]', '<>', (r'\{', r'\}'), (r'\lfloor', r'\rfloor'), (r'\lceil', r'\rceil')]
    ],
]

LIGHTWEIGHT_MATH_TESTS = [
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

_DIGITS = "0123456789"
_UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
_LOWERCASE = "abcdefghijklmnopqrstuvwxyz"
_UPPERGREEK = ("\\Gamma \\Delta \\Theta \\Lambda \\Xi \\Pi \\Sigma \\Upsilon \\Phi \\Psi "
               "\\Omega")
_LOWERGREEK = ("\\alpha \\beta \\gamma \\delta \\epsilon \\zeta \\eta \\theta \\iota "
               "\\lambda \\mu \\nu \\xi \\pi \\kappa \\rho \\sigma \\tau \\upsilon "
               "\\phi \\chi \\psi")
_ALL = [_DIGITS, _UPPERCASE, _LOWERCASE, _UPPERGREEK, _LOWERGREEK]
_BB = [_DIGITS, _UPPERCASE, _LOWERCASE, r'\Gamma \Pi \Sigma \gamma \pi']

FONT_TEST_SPECS = [
    ([], _ALL),
    (['mathrm'], _ALL),
    (['mathbf'], _ALL),
    (['mathit'], _ALL),
    (['mathtt'], [_DIGITS, _UPPERCASE, _LOWERCASE]),
    (['mathbb'], _BB),
    (['mathrm', 'mathbb'], _BB),
    (['mathbf', 'mathbb'], _BB),
    (['mathcal'], [_UPPERCASE]),
    (['mathfrak'], [_UPPERCASE, _LOWERCASE]),
    (['mathbf', 'mathfrak'], [_UPPERCASE, _LOWERCASE]),
    (['mathscr'], [_UPPERCASE, _LOWERCASE]),
    (['mathsf'], [_DIGITS, _UPPERCASE, _LOWERCASE]),
    (['mathrm', 'mathsf'], [_DIGITS, _UPPERCASE, _LOWERCASE]),
    (['mathbf', 'mathsf'], [_DIGITS, _UPPERCASE, _LOWERCASE]),
    (['mathbfit'], _ALL),
]


def _font_tests():
    tests = []
    for fonts, chars in FONT_TEST_SPECS:
        wrapper = ''.join([
            ' '.join(fonts), ' $', *(r'\%s{' % font for font in fonts), '%s',
            *('}' for _ in fonts), '$',
        ])
        tests.extend(wrapper % font_set for font_set in chars)
    return tests


FONT_TESTS = _font_tests()

# The expressions of the module's figure-equality and smoke tests.
FIGURE_TEST_EXPRESSIONS = [
    # test_short_long_accents
    r"$\"a\`a\'a\~a\.a\^a$", r"$\ddot a\grave a\acute a\tilde a\dot a\hat a$",
    # test_single_minus_sign
    r"$-$",
    # test_spaces
    r"$1\,2\>3\ 4$", r"$1\/2\:3~4$",
    # test_operator_space
    r"$\log 6$", r"$\log(6)$", r"$\arcsin 6$", r"$\arcsin|6|$", r"$\operatorname{op} 6$",
    r"$\operatorname{op}[6]$", r"$\cos^2$", r"$\log_2$", r"$\sin^2 \max \cos$",
    r"$\mathrm{log\,}6$", r"$\mathrm{log}(6)$", r"$\mathrm{arcsin\,}6$",
    r"$\mathrm{arcsin}|6|$", r"$\mathrm{op\,}6$", r"$\mathrm{op}[6]$", r"$\mathrm{cos}^2$",
    r"$\mathrm{log}_2$", r"$\mathrm{sin}^2 \mathrm{\,max} \mathrm{\,cos}$",
    # test_inverted_delimiters
    r"$\left)\right($", r"$)($",
    # test_genfrac_displaystyle (with a fixed rule thickness)
    r"$\dfrac{2x}{3y}$", r"$\genfrac{}{}{0.5}{0}{2x}{3y}$",
    # test_mathtext_fallback, test_math_fontfamily, test_default_math_fontfamily
    r"a$A\AA\breve\gimel$", r"$This\ text\ should\ have\ one\ font$", r"abc$abc\alpha$",
    # test_mathtext_single_char_super_with_prime
    r"$x^2'$", r"$x^a'$", r"$x^2''$", r"$x^\alpha'$",
    # test_text_nested_braces, test_text_escaped_braces
    r"$\text{{example}}$", r"$\text{a{b}{{c}}d}$", r"$\text{example}$", r"$\text{abcd}$",
    r"$\text{{\{example\}}}$", r"$\text{\{example\}}$",
    # test_boldsymbol, test_mathnormal
    r"$\boldsymbol{\mathrm{abc0123\alpha}}$", r"$\mathrm{abc0123\alpha}$",
    r"$\mathnormal{0123456789}$", r"$\mathrm{0123456789}$",
    # test_phantoms
    r"$\rlap{rlap}extra$", r"$rlap$", r"$extra$", r"$extra\llap{llap}$", r"$llap$",
    r"$\phantom{phantom}$",
    # test_box_repr
    r"$\frac{1}{2}$",
]

# test_mathtext_operators, one expression per row of its symbol table.
OPERATOR_ROWS = [
    r"\increment \smallin \notsmallowns",
    r"\smallowns \QED \rightangle",
    r"\smallintclockwise \smallvarointclockwise",
    r"\smallointctrcclockwise",
    r"\ratio \minuscolon \dotsminusdots",
    r"\sinewave \simneqq \nlesssim",
    r"\ngtrsim \nlessgtr \ngtrless",
    r"\cupleftarrow \oequal \rightassert",
    r"\rightModels \hermitmatrix \barvee",
    r"\measuredrightangle \varlrtriangle",
    r"\equalparallel \npreccurlyeq \nsucccurlyeq",
    r"\nsqsubseteq \nsqsupseteq \sqsubsetneq",
    r"\sqsupsetneq  \disin \varisins",
    r"\isins \isindot \varisinobar",
    r"\isinobar \isinvb \isinE",
    r"\nisd \varnis \nis",
    r"\varniobar \niobar \bagmember",
    r"\triangle",
]
OPERATOR_TESTS = ["$" + " ".join("{%s}" % op for op in row.split()) + "$"
                  for row in OPERATOR_ROWS]

# test_mathtext_exceptions (every one a ValueError), plus escapes and an
# empty \genfrac rule size the grammar rejects, and \boldsymbol around a
# script, on which matplotlib's parse action crashes with an IndexError.
INVALID = [
    r'$\hspace{}$', r'$\hspace{foo}$', r'$\sinx$', r'$\dotx$', r'$\frac$', r'$\frac{}{}$',
    r'$\binom$', r'$\binom{}{}$', r'$\genfrac$', r'$\genfrac{}{}{}{}{}{}$', r'$\sqrt$',
    r'$\sqrt f$', r'$\overline$', r'$\overline{}$', r'$\leftF$', r'$\rightF$',
    r'$\left(\right$', r'$\left($', r'$\dfrac$', r'$\dfrac{}{}$', r'$\overset$',
    r'$\underset$', r'$\foo$', r'$a^2^2$', r'$a_2_2$', r'$a^2_a^2$', r'$a = {b$',
    r'$\text$', r'$\text{foo$',
    r'$\&$', r'$\ldotp$', r'$\genfrac{(}{)}{}{0}{x}{y}$', r'$\boldsymbol{\sum_i x_i}$',
]
# Symbols the grammar accepts without a Unicode mapping: the DejaVu fontsets
# raise "Unknown symbol", the others draw their dummy symbol.
UNMAPPED = [r'$\[$', r'$\]$', r'$a\[b\]c$']

EXPRESSIONS = [
    *MATH_TESTS, *SVGASTEXT_MATH_TESTS, *LIGHTWEIGHT_MATH_TESTS, *FONT_TESTS,
    *FIGURE_TEST_EXPRESSIONS, *OPERATOR_TESTS,
]

# --------------------------------------------------------------------------
# Systematic sweep over the parser's tables
# --------------------------------------------------------------------------

SWEEP_SIZE = (12.0, 100.0)
_P = _mathtext.Parser
# One body per \left height step; the tallest needs extensible delimiters.
_HEIGHTS = [
    "x", r"\frac{a}{b}", r"\frac{\frac{a}{b}}{c}", r"\frac{\frac{a}{b}}{\frac{c}{d}}",
    r"\frac{\frac{\frac{a}{b}}{c}}{\frac{d}{\frac{e}{f}}}",
]

MISC_SWEEP = [
    # fractions
    r"$\frac{a}{b} \dfrac{a}{b} \binom{a}{b} x^{\frac{a}{b}} x_{\dfrac{a}{b}} \frac{x^2}{y_1}$",
    r"$\genfrac{(}{)}{0}{0}{a}{b} \genfrac{[}{]}{1}{1}{a}{b} \genfrac{\{}{\}}{2}{2}{a}{b}$",
    r"$\genfrac{|}{\|}{0.5}{3}{a}{b} \genfrac{\langle}{\rangle}{0.2}{0}{x}{y} \genfrac{}{}{3}{}{x}{y}$",
    r"$\binom{\binom{a}{b}}{c} \dfrac{\dfrac{\dfrac{1}{2}}{3}}{4} \frac{\int_0^1 f}{\sum_i g}$",
    # radicals
    r"$\sqrt{x} \sqrt[3]{x} \sqrt[n]{\frac{a}{b}} \sqrt{\dfrac{\dfrac{a}{b}}{c}} \sqrt[10]{\sqrt[3]{y}}$",
    r"$x^{\sqrt{2}} \sqrt{\left(\frac{a}{b}\right)} \sqrt{\sqrt{\sqrt{\sqrt{x}}}} \sqrt[\alpha]{\beta}$",
    # stacking
    r"$\overset{a}{\underset{b}{=}} \overset{\frac{a}{b}}{X} \underset{\sum}{x} \overline{\frac{a}{b}} \underline{x_1^2}$",
    r"$\sum_{\substack{a\\b\\c\\d}} x \prod_{\substack{i<j\\j<k}}^{\substack{n\\m}} y$",
    r"$\overline{\overline{\overline{x}}} \underline{\underline{y}} \overset{\overset{a}{b}}{c} \underset{x}{\underset{y}{z}}$",
    # scripts
    r"$f' f'' f''' f'_1 f'^2 x^{y^{z^{w}}} x_{a_{b_{c_{d_{e_{f_{g}}}}}}} {}^{14}_{6}C$",
    r"$x^{2^{2^{2^{2^{2^{2^{2}}}}}}} \int_0^1 \oint_C \alpha_1^2 A^\prime_i$",
    r"$\Gamma_{ij}^k f_x^y \left(x\right)^2_i \frac{a}{b}^2 \sqrt{x}_3 \hat{x}^2 x^{-1}_{+1}$",
    # operator names, phantoms, text mode, escapes
    r"$\operatorname{argmax}_x \operatorname{Tr}^2 \operatorname{sinc}(x) \operatorname{d}\!x \operatorname{a\,b}$",
    r"$\phantom{x}y \rlap{ab}c \llap{ab}c \phantom{\frac{a}{b}} x\phantom{yy}z$",
    r"$\text{a b  c} \text{\{x\}} x\text{-}y \mathrm{a\ b} \mathbf{\text{bold}}$",
    r"$\$ \% \# \_ \{ \} \backslash \|$",
    r"$a+b-c*d/e<f>g=h:i,j.k;l!m?n&o@p(q)r[s]t|u$",
    r"$\boldsymbol{a+b} \boldsymbol{\Gamma\alpha 1} \boldsymbol{\mathrm{x}\frac{a}{b}}$",
    # kerning in every style
    r"$AV To Wa yv LT \mathrm{AV To Wa yv LT} \mathbf{AV To Wa} \mathsf{AV To} \mathtt{AV To}$",
    # binary operators and relations in context
    r"$-x +x a-b a=-b (-x) \{-1\} x^{-y} [+1] a\times-b a,-b$",
    r"$a<b>c a\leq b \geq c a\to b \mapsto c \Rightarrow d a\pm b\mp c$",
    # non-ASCII input, fallbacks and the last resort
    "$αβγ ∑∫∏ →≤≥ x² ℝℂ ÅÆØ ΓΔ x×y a−b ∂f ℏ €£¥$",
    "$中 😀 ʘ ᴀ ⅀ ∯$",
    r"$\mathcal{\alpha} \mathfrak{\alpha} \mathscr{\alpha} \mathbb{\alpha} \mathbfit{\alpha} \mathsf{\Gamma}$",
    r"$\mathbb{\Gamma\pi} \mathtt{\{x\}} \mathbf{\partial} \mathsf{\partial\nabla} \mathit{\Gamma\alpha}$",
    # \middle
    r"$\left(\frac{a}{b}\middle|\frac{c}{d}\middle\|\frac{e}{f}\middle/\frac{g}{h}\middle\backslash x\right)$",
    r"$\left\langle\dfrac{a}{b}\middle\vert x\middle\Vert\frac{\frac{a}{b}}{c}\middle\uparrow y\right\rangle$",
    # dots
    r"$\ldots \cdots \vdots \ddots \dots a\ldots b a\cdots b \cdotp$",
]


def _parses(s):
    fonts = _mathtext.DejaVuSansFonts(FontProperties(), LoadFlags.NO_HINTING)
    try:
        _P().parse(s, fonts, 12, 100)
    except ValueError:
        return False
    return True


def _symbol_sweep():
    from matplotlib._mathtext_data import tex2uni

    names = [rf"\{name}" for name in sorted(tex2uni) if _parses(rf"$\{name}$")]
    return ["$" + " ".join(names[i:i + 12]) + "$" for i in range(0, len(names), 12)]


def _structure_sweep():
    accents = [rf"$\{a}{{x}} \{a}{{A}} \{a}{{\imath}} \{a}{{\alpha}}$"
               for a in sorted(_P._accent_map) if len(a) > 1]
    accents += [rf"$\{a}{{x}} \{a}{{xyz}} \{a}{{ABCDEFGH}}$" for a in sorted(_P._wide_accents)]
    sample = r"Az09\alpha\Gamma+=(\partial\infty"
    fonts = [rf"$\math{f}{{{sample}}}$" for f in sorted(_P._fontnames)]
    fonts += [rf"$\{f} {sample}$" for f in sorted(_P._fontnames)]
    functions = [rf"$\{f} x \{f}_a^b x \{f}(x)$" for f in sorted(_P._function_names)]
    limits = [rf"${s} x {s}_a x {s}^b x {s}_{{i=0}}^{{n}} x$"
              for s in sorted(_P._overunder_symbols | _P._dropsub_symbols)]
    delims = ["$" + " ".join(rf"\left{d} {h} \right." for h in _HEIGHTS) + "$"
              for d in sorted(_P._delims - {"."})]
    spaces = ["$" + "".join(f"a{w}" for w in sorted(_P._space_widths)) + "b$",
              r"$a\hspace{1}b\hspace{-0.5}c\hspace{0.25}d\hspace{2.5}e\hspace{0}f$"]
    return [*accents, *fonts, *functions, *limits, *delims, *spaces, *MISC_SWEEP]


@pytest.fixture(scope="module", autouse=True)
def _bundled_fonts_only():
    with bundled_fonts_only():
        yield


_PARSER = _mathtext.Parser()


def _typeset(s, fontset, size, dpi):
    with matplotlib.rc_context({"mathtext.fontset": fontset}):
        MathTextParser._parse_cached.cache_clear()
        prop = FontProperties(size=size)
        vec = MathTextParser("path").parse(s, dpi=dpi, prop=prop)
        fonts_class = MathTextParser._font_type_mapping[prop.get_math_fontfamily()]
        box = _PARSER.parse(s, fonts_class(prop, LoadFlags.NO_HINTING),
                            prop.get_size_in_points(), dpi)
    fonts = []
    glyphs = []
    for font, fontsize, code, _glyph, x, y in vec.glyphs:
        name = os.path.basename(font.fname)
        if name not in fonts:
            fonts.append(name)
        glyphs.append([fonts.index(name), float(fontsize), int(code), float(x), float(y)])
    return {
        "box": [box.width, box.height, box.depth],
        "extent": [float(vec.width), float(vec.height), float(vec.depth)],
        "fonts": fonts,
        "glyphs": glyphs,
        "rects": [[float(v) for v in rect] for rect in vec.rects],
    }


def parse(s, fontset, size, dpi):
    return probes.probe("mathtext.MathTextParser.parse", _typeset,
                        s=s, fontset=fontset, size=size, dpi=dpi)


@pytest.mark.parametrize("fontset", FONTSETS)
def test_expressions(fontset):
    assert len(set(EXPRESSIONS)) == len(EXPRESSIONS)
    for size, dpi in SIZES:
        for s in EXPRESSIONS:
            result = parse(s, fontset, size, dpi)
            assert not isinstance(result, Exception), (fontset, s, result)


@pytest.mark.parametrize("fontset", FONTSETS)
def test_sweep(fontset):
    size, dpi = SWEEP_SIZE
    sweep = [*_symbol_sweep(), *_structure_sweep()]
    assert len(set(sweep)) == len(sweep)
    for s in sweep:
        result = parse(s, fontset, size, dpi)
        assert not isinstance(result, Exception), (fontset, s, result)


@pytest.mark.parametrize("fontset", FONTSETS)
def test_invalid(fontset):
    for size, dpi in SIZES:
        for s in INVALID:
            assert isinstance(parse(s, fontset, size, dpi), (ValueError, IndexError)), (fontset, s)
        for s in UNMAPPED:
            result = parse(s, fontset, size, dpi)
            assert isinstance(result, ValueError) == fontset.startswith("dejavu"), (fontset, s)
