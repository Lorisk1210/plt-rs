//! The parametrised mathtext tests: the expressions of
//! `test_mathtext_rendering` (`math_tests`) and `test_mathfont_rendering`
//! (`font_tests`) the `mathtext` group does not draw, ten rows to a case
//! like that group's, in each of the tests' five fontsets, and
//! `test_mathtext_rendering_svgastext`.

use plt::figure::Figure;

use super::super::mathtext::{DELIMITERS, ROW, WIDTH, rows};
use crate::figures::FigureCase;

/// `math_tests_1`: the uncovered `math_tests` 1 to 10.
const MATH_TESTS_1: [&str; 10] = [
    r"$x\hspace{-0.2}\doteq\hspace{-0.2}y$",
    r"\$100.00 $\alpha \_$",
    r"$\frac{\$100.00}{y}$",
    r"$x   y$",
    r"$x+y\ x=y\ x<y\ x:y\ x,y\ x@y$",
    r"$100\%y\ x*y\ x/y x\$y$",
    r"$x\leftarrow y\ x\forall y\ x-y$",
    r"$x \sf x \bf x {\cal X} \rm x$",
    r"$\{ \rm braces \}$",
    r"$\left(x\right)$",
];

/// `math_tests_2`: the uncovered `math_tests` 11 to 20.
const MATH_TESTS_2: [&str; 10] = [
    r"$\sin(x)$",
    r"$x_2$",
    r"$x^2$",
    r"$x^2_y$",
    r"$x_y^2$",
    r"$dz/dt = \gamma x^2 + {\rm sin}(2\pi y+\phi)$",
    r"Variable $i$ is good",
    r"$\Delta_i^j$",
    r"$\Delta^j_{i+1}$",
    r"$\arccos((x^i))$",
];

/// `math_tests_3`: the uncovered `math_tests` 21 to 30.
const MATH_TESTS_3: [&str; 10] = [
    r"$\gamma = \frac{x=\frac{6}{8}}{y} \delta$",
    r"$f'\quad f'''(x)\quad ''/\mathrm{yr}$",
    r"$\frac{x_2888}{y}$",
    r"$\sqrt[3]{x}=5$",
    r"$\frac{X}{\frac{X}{Y}}$",
    r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2} \int^{\alpha_2}_{\alpha_2} d \alpha^\prime_2 \left[\frac{ U^{2\beta}_{\delta_1 \rho_1} - \alpha^\prime_2U^{1\beta}_{\rho_1 \sigma_2} }{U^{0\beta}_{\rho_1 \sigma_2}}\right]$",
    r"$\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega$",
    r"$\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi$",
    r"${x}^{2}{y}^{2}$",
    r"${}_{2}F_{3}$",
];

/// `math_tests_4`: the uncovered `math_tests` 31 to 40.
const MATH_TESTS_4: [&str; 10] = [
    r"$\frac{x+{y}^{2}}{k+1}$",
    r"$x+{y}^{\frac{2}{k+1}}$",
    r"$\frac{a}{b/2}$",
    r"${a}_{0}+\dfrac{1}{{a}_{1}+\dfrac{1}{{a}_{2}+\dfrac{1}{{a}_{3}+\dfrac{1}{{a}_{4}}}}}$",
    r"$\binom{n}{k/2}$",
    r"${x}^{2y}$",
    r"$\sum _{i=1}^{p}\sum _{j=1}^{q}\sum _{k=1}^{r}{a}_{ij}{b}_{jk}{c}_{ki}$",
    r"${2}^{{2}^{{2}^{x}}}$",
    r"${\int }_{1}^{x}\frac{\mathrm{dt}}{t}$",
    r"$\int {\int }_{D}\mathrm{dx} \mathrm{dy}$",
];

/// `math_tests_5`: the uncovered `math_tests` 41 to 50.
const MATH_TESTS_5: [&str; 10] = [
    r"${y}_{{x}^{2}}$",
    r"${y}_{{x}_{2}}$",
    r"${x}_{92}^{31415}+\pi $",
    r"${y}_{3}^{\prime \prime \prime }$",
    r"$\left( \xi \left( 1 - \xi \right) \right)$",
    r"$\left(2 \, a=b\right)$",
    r"$? ! &$",
    r"$\mathring{A}  \AA$",
    r"$\hspace{-0.2}\dotplus\hspace{-0.2}$ $\hspace{-0.2}\doteq\hspace{-0.2}$ $\hspace{-0.2}\doteqdot\hspace{-0.2}$ $\ddots$",
    r"$xyz^kx_kx^py^{p-2} d_i^jb_jc_kd x^j_i E^0 E^0_u$",
];

/// `math_tests_6`: the uncovered `math_tests` 51 to 58.
const MATH_TESTS_6: [&str; 8] = [
    r"${xyz}^k{x}_{k}{x}^{p}{y}^{p-2} {d}_{i}^{j}{b}_{j}{c}_{k}{d} {x}^{j}_{i}{E}^{0}{E}^0_u$",
    r"${\int}_x^x x\oint_x^x x\int_{X}^{X}x\int_x x \int^x x \int_{x} x\int^{x}{\int}_{x} x{\int}^{x}_{x}x$",
    r"testing$^{123}$",
    r"$\overline{\omega}^x \frac{1}{2}_0^x$",
    r"$,$ $.$ $1{,}234{, }567{ , }890$ and $1,234,567,890$",
    r"$\left(X\right)_{a}^{b}$",
    r"$\dfrac{\$100.00}{y}$",
    r"$a=-b-c$",
];

/// `font_tests_1`: the uncovered `font_tests` 1 to 10.
const FONT_TESTS_1: [&str; 10] = [
    r" $0123456789$",
    r" $ABCDEFGHIJKLMNOPQRSTUVWXYZ$",
    r" $abcdefghijklmnopqrstuvwxyz$",
    r" $\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega$",
    r" $\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi$",
    r"mathrm $\mathrm{0123456789}$",
    r"mathrm $\mathrm{abcdefghijklmnopqrstuvwxyz}$",
    r"mathrm $\mathrm{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathrm $\mathrm{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
    r"mathbf $\mathbf{0123456789}$",
];

/// `font_tests_2`: the uncovered `font_tests` 11 to 20.
const FONT_TESTS_2: [&str; 10] = [
    r"mathbf $\mathbf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbf $\mathbf{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbf $\mathbf{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathit $\mathit{0123456789}$",
    r"mathit $\mathit{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathit $\mathit{abcdefghijklmnopqrstuvwxyz}$",
    r"mathit $\mathit{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
    r"mathtt $\mathtt{0123456789}$",
    r"mathtt $\mathtt{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbb $\mathbb{0123456789}$",
];

/// `font_tests_3`: the uncovered `font_tests` 21 to 30.
const FONT_TESTS_3: [&str; 10] = [
    r"mathbb $\mathbb{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbb $\mathbb{\Gamma \Pi \Sigma \gamma \pi}$",
    r"mathrm mathbb $\mathrm{\mathbb{0123456789}}$",
    r"mathrm mathbb $\mathrm{\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathrm mathbb $\mathrm{\mathbb{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathbb $\mathbf{\mathbb{0123456789}}$",
    r"mathbf mathbb $\mathbf{\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathbb $\mathbf{\mathbb{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathbb $\mathbf{\mathbb{\Gamma \Pi \Sigma \gamma \pi}}$",
    r"mathfrak $\mathfrak{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
];

/// `font_tests_4`: the uncovered `font_tests` 31 to 40.
const FONT_TESTS_4: [&str; 10] = [
    r"mathbf mathfrak $\mathbf{\mathfrak{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathfrak $\mathbf{\mathfrak{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathscr $\mathscr{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathsf $\mathsf{0123456789}$",
    r"mathsf $\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathsf $\mathsf{abcdefghijklmnopqrstuvwxyz}$",
    r"mathrm mathsf $\mathrm{\mathsf{0123456789}}$",
    r"mathrm mathsf $\mathrm{\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathrm mathsf $\mathrm{\mathsf{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbf mathsf $\mathbf{\mathsf{0123456789}}$",
];

/// `font_tests_5`: the uncovered `font_tests` 41 to 47.
const FONT_TESTS_5: [&str; 7] = [
    r"mathbf mathsf $\mathbf{\mathsf{ABCDEFGHIJKLMNOPQRSTUVWXYZ}}$",
    r"mathbf mathsf $\mathbf{\mathsf{abcdefghijklmnopqrstuvwxyz}}$",
    r"mathbfit $\mathbfit{0123456789}$",
    r"mathbfit $\mathbfit{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbfit $\mathbfit{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbfit $\mathbfit{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathbfit $\mathbfit{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
];

/// The cases drawing `$expressions` in each fontset of the tests, named
/// `mpl_text/<$name>_<fontset>`.
macro_rules! per_fontset {
    ($name:literal, $expressions:ident) => {
        per_fontset!($name, $expressions, ["cm", "stix", "stixsans", "dejavusans", "dejavuserif"])
    };
    ($name:literal, $expressions:ident, [$($fontset:literal),+]) => {
        [$(
            FigureCase::new(concat!("mpl_text/", $name, "_", $fontset), |fig| {
                rows(fig, &$expressions);
                Ok(())
            })
            .sized(WIDTH, ROW * $expressions.len() as f64, 100.0)
            .with_rc(&[("mathtext.fontset", $fontset)])
        ),+]
    };
}

/// `test_mathtext_rendering_svgastext`: the `mathtext` group's delimiter
/// rows in 16 pt (saved with SVG text as text). The test hides the figure
/// background, which the case keeps (see the Python module).
fn svgastext(fig: &Figure) {
    for text in rows(fig, &DELIMITERS) {
        text.set_fontsize(16.0);
    }
}

/// `svgastext_<fontset>`, in the test's two fontsets.
const SVGASTEXT: [FigureCase; 2] = [
    FigureCase::new("mpl_text/svgastext_cm", |fig| {
        svgastext(fig);
        Ok(())
    })
    .sized(WIDTH, ROW * DELIMITERS.len() as f64, 100.0)
    .with_rc(&[("mathtext.fontset", "cm"), ("svg.fonttype", "none")]),
    FigureCase::new("mpl_text/svgastext_dejavusans", |fig| {
        svgastext(fig);
        Ok(())
    })
    .sized(WIDTH, ROW * DELIMITERS.len() as f64, 100.0)
    .with_rc(&[("mathtext.fontset", "dejavusans"), ("svg.fonttype", "none")]),
];

/// Every case of the module.
pub(super) fn cases() -> Vec<FigureCase> {
    let mut cases = [
        per_fontset!("math_tests_1", MATH_TESTS_1),
        per_fontset!("math_tests_2", MATH_TESTS_2),
        per_fontset!("math_tests_3", MATH_TESTS_3),
        per_fontset!("math_tests_4", MATH_TESTS_4),
        per_fontset!("math_tests_5", MATH_TESTS_5),
        per_fontset!("math_tests_6", MATH_TESTS_6),
        per_fontset!("font_tests_1", FONT_TESTS_1),
        per_fontset!("font_tests_2", FONT_TESTS_2),
        per_fontset!("font_tests_3", FONT_TESTS_3),
        per_fontset!("font_tests_4", FONT_TESTS_4),
        per_fontset!("font_tests_5", FONT_TESTS_5),
    ]
    .concat();
    cases.extend(SVGASTEXT);
    cases
}
