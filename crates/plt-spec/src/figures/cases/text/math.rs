//! Mathtext and unicode cases of `figures/cases_text.py`.

use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::text::VAlign;

use super::blank;
use crate::figures::FigureCase;

/// `MATH_SAMPLES`: one line per mathtext feature family.
const MATH_SAMPLES: [&str; 8] = [
    r"$\frac{a}{b} + \dfrac{x^2}{y_1} - \sqrt{2} \cdot \sqrt[3]{x+1} \binom{n}{k} \frac{\frac{1}{2}}{z}$",
    r"$\sum_{i=0}^{\infty} x_i \quad \int_0^1 f(x)\,dx \quad \prod_{k=1}^{n} k \quad \oint_C \quad \lim_{x \to 0}$",
    r"$\hat{a}\ \bar{b}\ \vec{c}\ \dot{d}\ \ddot{e}\ \tilde{f}\ \acute{g}\ \breve{i}\ \widehat{xyz}\ \overline{AB}$",
    r"$\left( \frac{1}{2} \right) \left[ \frac{a}{b} \right] \left\{ \frac{\alpha}{\beta} \right\} \left| x \right| \left\langle \psi \right\rangle$",
    r"$a\!b\ |\ a\,b\ |\ a\:b\ |\ a\;b\ |\ a\quad b\ |\ a\qquad b$",
    r"$\mathrm{Rm}\ \mathit{It}\ \mathbf{Bf}\ \mathcal{CAL}\ \mathbb{RNZ}\ \mathfrak{Frak}\ \mathsf{Sf}\ \mathtt{Tt}$",
    r"$\alpha\beta\gamma\delta\epsilon\theta\lambda\pi\Gamma\Delta\Omega\ \leq\geq\neq\approx\infty\partial\nabla\pm\times\rightarrow\Leftrightarrow$",
    r"$x_{i,j}^{2n+1}\ e^{-x^2/2}\ 10^{-3}\ {}^{14}\mathrm{C}\ \mathrm{sin}^2\theta\ \operatorname{erf}(x)\ \underline{u}$",
];

/// `_math_case(fontset)`: every sample as figure text, top aligned.
fn math_samples(fig: &Figure) {
    for (i, sample) in MATH_SAMPLES.into_iter().enumerate() {
        fig.text(0.02, 0.97 - 0.12 * i as f64, sample)
            .set_fontsize(13.0)
            .set_valign(VAlign::Top);
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("text/mathtext_dejavusans", |fig| {
        math_samples(fig);
        Ok(())
    })
    .sized(7.0, 6.0, 100.0)
    .with_rc(&[("mathtext.fontset", "dejavusans")]),
    FigureCase::new("text/mathtext_dejavuserif", |fig| {
        math_samples(fig);
        Ok(())
    })
    .sized(7.0, 6.0, 100.0)
    .with_rc(&[("mathtext.fontset", "dejavuserif")]),
    FigureCase::new("text/mathtext_cm", |fig| {
        math_samples(fig);
        Ok(())
    })
    .sized(7.0, 6.0, 100.0)
    .with_rc(&[("mathtext.fontset", "cm")]),
    FigureCase::new("text/mathtext_stix", |fig| {
        math_samples(fig);
        Ok(())
    })
    .sized(7.0, 6.0, 100.0)
    .with_rc(&[("mathtext.fontset", "stix")]),
    FigureCase::new("text/mathtext_stixsans", |fig| {
        math_samples(fig);
        Ok(())
    })
    .sized(7.0, 6.0, 100.0)
    .with_rc(&[("mathtext.fontset", "stixsans")]),
    FigureCase::new("text/mathtext_title_labels", |fig| {
        let ax = fig.subplot();
        let x = linspace(0.0, 2.0, 30);
        let y: Vec<f64> = x.iter().map(|x| (-(x * x)).exp()).collect();
        ax.plot_with(&x, y, LineProps::default().label(r"$e^{-x^2}$"))?;
        ax.set_title(
            r"$\alpha_i > \beta_i \quad \sum_{n=1}^\infty \frac{1}{n^2} = \frac{\pi^2}{6}$",
        );
        ax.set_xlabel(r"time $t$ [$\mu$s]")
            .set_ylabel(r"$\sqrt{\langle x^2 \rangle}$");
        ax.xaxis().set_ticks([0.0, 1.0, 2.0]).set_ticklabels([
            r"$0$",
            r"$\frac{1}{2}\pi$",
            r"$\pi$",
        ])?;
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("text/mathtext_mixed_sizes", |fig| {
        let ax = blank(fig)?;
        ax.set_axis_off();
        for (i, size) in [8, 12, 18, 26].into_iter().enumerate() {
            ax.text(
                0.2,
                9.0 - 2.4 * i as f64,
                r"Area $A = \pi r^2$, $\int_a^b g$",
            )
            .set_fontsize(f64::from(size))
            .set_valign(VAlign::Top);
        }
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("text/unicode", |fig| {
        let ax = blank(fig)?;
        ax.set_axis_off();
        let lines = [
            (8.5, "Greek: αβγδε ΓΔΘΛΞΠΣΦΨΩ"),
            (6.5, "Latin: àáâãäå çèéêë ñ øœ ß ÀÉÎÕÜ Łódź"),
            (4.5, "Arrows: ← → ↑ ↓ ↔ ⇐ ⇒ ⇔"),
            (2.5, "Symbols: ∞ ± × ÷ ≤ ≥ ≠ ≈ ∑ ∫ √ °C € £ ¥"),
        ];
        for (y, line) in lines {
            ax.text(0.2, y, line).set_fontsize(13.0);
        }
        ax.text(0.2, 0.5, "Cyrillic: Привет мир")
            .set_fontsize(13.0)
            .set_family(["serif"]);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
