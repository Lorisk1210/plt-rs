//! Math text: accents, display and text style fractions, texts turned
//! into images, the mathtext demo and feature overview, math font
//! families per text and the STIX-style alphabets (`accented_text`,
//! `dfrac_demo`, `mathtext_asarray`, `mathtext_demo`,
//! `mathtext_examples`, `mathtext_fontfamily_example` and
//! `stix_fonts_demo`).

use plt::figure::{BboxInches, SaveOptions};
use plt::fonts::{FontSize, FontWeight, Fontset};
use plt::image::FigimageOptions;
use plt::layout::TightLayout;
use plt::legend::LegendOptions;
use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::{AnnotateOptions, HAlign, VAlign};
use plt::transforms::Transform;

use crate::figures::{CaseResult, FigureCase};

fn accented_text_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y((0..10).map(f64::from).collect::<Vec<_>>());
    ax.set_title(r"$\ddot{o}\acute{e}\grave{e}\hat{O}\breve{i}\bar{A}\tilde{n}\vec{q}$")
        .set_fontsize(20.0);
    ax.set_xlabel(r#"$\"o\ddot o \'e\`e\~n\.x\^y$"#);
    ax.xaxis().set_label_fontsize(FontSize::Points(20.0));
    ax.text(4.0, 0.5, r"$F=m\ddot{x}$");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn dfrac_demo(fig: &Figure) -> CaseResult {
    for (y, text) in [
        (0.3, r"\dfrac: $\dfrac{a}{b}$"),
        (0.7, r"\frac: $\frac{a}{b}$"),
    ] {
        fig.text(0.5, y, text)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
    }
    Ok(())
}

/// `text_to_rgba`: `text` drawn alone on a transparent figure, saved at
/// `dpi` cropped to the text, as 8-bit RGBA pixels.
fn text_to_rgba(
    text: &str,
    color: &str,
    dpi: f64,
) -> Result<Grid<[u8; 4]>, Box<dyn std::error::Error>> {
    let fig = Figure::new();
    fig.set_facecolor(Color::from("none").resolve()?);
    fig.text(0.0, 0.0, text).set_color(color).set_fontsize(20.0);
    let image = fig
        .render_raster_with(SaveOptions {
            dpi: Some(dpi),
            bbox_inches: Some(BboxInches::Tight),
            pad_inches: Some(0.0),
            ..SaveOptions::default()
        })?
        .to_image();
    let (rows, cols) = (image.height(), image.width());
    Ok(Grid::from_fn(rows, cols, |row, col| {
        image.pixel(col, row).unwrap_or_default()
    }))
}

fn mathtext_asarray(fig: &Figure) -> CaseResult {
    let texts = [
        (r"IQ: $\sigma_i=15$", "blue", 50.0),
        ("some other string", "red", 150.0),
    ];
    for (text, color, y) in texts {
        let pixels = text_to_rgba(text, color, 200.0)?;
        fig.figimage(
            pixels,
            FigimageOptions {
                xo: 100.0,
                yo: y,
                ..FigimageOptions::default()
            },
        )?;
    }
    for (text, color, y) in texts {
        fig.text(100.0, y + 200.0, text)
            .set_color(color)
            .set_fontsize(20.0)
            .set_transform(Transform::Display);
    }
    Ok(())
}

fn mathtext_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y([1.0, 2.0, 3.0]).set_label(r"$\sqrt{x^2}$");
    ax.legend(LegendOptions::default())?;
    ax.set_xlabel(r"$\Delta_i^j$");
    ax.xaxis().set_label_fontsize(FontSize::Points(20.0));
    ax.set_ylabel(r"$\Delta_{i+1}^j$");
    ax.yaxis().set_label_fontsize(FontSize::Points(20.0));
    ax.set_title(r"$\Delta_i^j \hspace{0.4} \mathrm{versus} \hspace{0.4} \Delta_{i+1}^j$")
        .set_fontsize(20.0);
    ax.text(
        1.0,
        1.6,
        r"$\mathcal{R}\prod_{i=\alpha_{i+1}}^\infty a_i\sin(2 \pi f x_i)$",
    )
    .set_fontsize(20.0)
    .set_valign(VAlign::Bottom);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The formulas of `mathtext_examples`, by feature; the first one heads
/// the figure.
const MATHTEXT_DEMOS: [(&str, &str); 8] = [
    (
        "Header demo",
        concat!(
            r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = ",
            r"U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2} ",
            r"\int^{\alpha_2}_{\alpha_2} d \alpha^\prime_2 \left[\frac{ ",
            r"U^{2\beta}_{\delta_1 \rho_1} - \alpha^\prime_2U^{1\beta}_",
            r"{\rho_1 \sigma_2} }{U^{0\beta}_{\rho_1 \sigma_2}}\right]$",
        ),
    ),
    (
        "Subscripts and superscripts",
        concat!(
            r"$\alpha_i > \beta_i,\ ",
            r"\alpha_{i+1}^j = {\rm sin}(2\pi f_j t_i) e^{-5 t_i/\tau},\ ",
            r"\ldots$",
        ),
    ),
    (
        "Fractions, binomials and stacked numbers",
        concat!(
            r"$\frac{3}{4},\ \binom{3}{4},\ \genfrac{}{}{0}{}{3}{4},\ ",
            r"\left(\frac{5 - \frac{1}{x}}{4}\right),\ \ldots$",
        ),
    ),
    ("Radicals", r"$\sqrt{2},\ \sqrt[3]{x},\ \ldots$"),
    (
        "Fonts",
        concat!(
            r"$\mathrm{Roman}\ , \ \mathit{Italic}\ , \ \mathtt{Typewriter} \ ",
            r"\mathrm{or}\ \mathcal{CALLIGRAPHY}$",
        ),
    ),
    (
        "Accents",
        concat!(
            r"$\acute a,\ \bar a,\ \breve a,\ \dot a,\ \ddot a, \ \grave a, \ ",
            r"\hat a,\ \tilde a,\ \vec a,\ \widehat{xyz},\ \widetilde{xyz},\ ",
            r"\ldots$",
        ),
    ),
    (
        "Greek, Hebrew",
        concat!(
            r"$\alpha,\ \beta,\ \chi,\ \delta,\ \lambda,\ \mu,\ ",
            r"\Delta,\ \Gamma,\ \Omega,\ \Phi,\ \Pi,\ \Upsilon,\ \nabla,\ ",
            r"\aleph,\ \beth,\ \daleth,\ \gimel,\ \ldots$",
        ),
    ),
    (
        "Delimiters, functions and Symbols",
        concat!(
            r"$\coprod,\ \int,\ \oint,\ \prod,\ \sum,\ ",
            r"\log,\ \sin,\ \approx,\ \oplus,\ \star,\ \varpropto,\ ",
            r"\infty,\ \partial,\ \Re,\ \leftrightsquigarrow, \ \ldots$",
        ),
    ),
];

fn mathtext_examples(fig: &Figure) -> CaseResult {
    let grey = Color::from((51.0 / 255.0, 51.0 / 255.0, 51.0 / 255.0));
    let ax = fig.add_axes([0.01, 0.01, 0.98, 0.90]);
    ax.set_facecolor("white").set_frame_on(true);
    ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    ax.set_title("Matplotlib's math rendering engine")
        .set_color(grey.clone())
        .set_fontsize(14.0)
        .set_weight(FontWeight::BOLD);
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    #[expect(clippy::cast_precision_loss, reason = "eight lines")]
    let line_axesfrac = 1.0 / MATHTEXT_DEMOS.len() as f64;
    let (_, header) = MATHTEXT_DEMOS[0];
    ax.annotate(
        header,
        (0.5, 1.0 - 0.59 * line_axesfrac),
        AnnotateOptions::default(),
    )?
    .set_color("tab:orange")
    .set_halign(HAlign::Center)
    .set_fontsize(20.0);
    for (i_line, (title, demo)) in MATHTEXT_DEMOS.iter().enumerate().skip(1) {
        #[expect(clippy::cast_precision_loss, reason = "eight lines")]
        let baseline = 1.0 - i_line as f64 * line_axesfrac;
        let baseline_next = baseline - line_axesfrac;
        let fill_color = ["white", "tab:blue"][i_line % 2];
        ax.axhspan(baseline, baseline_next)
            .set_color(fill_color)
            .set_alpha(Some(0.2));
        ax.annotate(
            format!("{title}:"),
            (0.06, baseline - 0.3 * line_axesfrac),
            AnnotateOptions::default(),
        )?
        .set_color(grey.clone())
        .set_weight(FontWeight::BOLD);
        ax.annotate(
            *demo,
            (0.04, baseline - 0.75 * line_axesfrac),
            AnnotateOptions::default(),
        )?
        .set_color(grey.clone())
        .set_fontsize(16.0);
    }
    Ok(())
}

fn mathtext_fontfamily_example(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_with(
        (0..11).map(f64::from).collect::<Vec<_>>(),
        (0..11).map(f64::from).collect::<Vec<_>>(),
        LineProps::default().color("0.9"),
    )?;
    let msg = r"Normal Text. $Text\ in\ math\ mode:\ \int_{0}^{\infty } x^2 dx$";
    for (y, fontset) in [(7.0, Fontset::Cm), (3.0, Fontset::DejaVuSerif)] {
        ax.text(1.0, y, msg)
            .set_fontsize(12.0)
            .set_math_fontfamily(fontset);
    }
    ax.set_title(r"$Title\ in\ math\ mode:\ \int_{0}^{\infty } x^2 dx$")
        .set_math_fontfamily(Fontset::StixSans)
        .set_fontsize(14.0);
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn stix_fonts_demo(fig: &Figure) -> CaseResult {
    let circle123 = "\u{2460}\u{2461}\u{2462}";
    let first = format!(r"${circle123}\;\mathrm{{{circle123}}}\;\mathbf{{{circle123}}}$");
    let tests = [
        first.as_str(),
        concat!(
            r"$\mathsf{Sans \Omega}\;\mathrm{\mathsf{Sans \Omega}}\;",
            r"\mathbf{\mathsf{Sans \Omega}}$",
        ),
        r"$\mathtt{Monospace}$",
        r"$\mathcal{CALLIGRAPHIC}$",
        r"$\mathbb{Blackboard\;\pi}$",
        r"$\mathrm{\mathbb{Blackboard\;\pi}}$",
        r"$\mathbf{\mathbb{Blackboard\;\pi}}$",
        r"$\mathfrak{Fraktur}\;\mathbf{\mathfrak{Fraktur}}$",
        r"$\mathscr{Script}$",
    ];
    #[expect(clippy::cast_precision_loss, reason = "nine lines")]
    let rows = tests.len() as f64;
    for (i, text) in tests.iter().rev().enumerate() {
        #[expect(clippy::cast_precision_loss, reason = "nine lines")]
        let y = (i as f64 + 0.5) / rows;
        fig.text(0.0, y, *text).set_fontsize(32.0);
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_text/accented_text_1", accented_text_1),
    FigureCase::new("gallery_text/dfrac_demo", dfrac_demo).sized(5.25, 0.75, 100.0),
    FigureCase::new("gallery_text/mathtext_asarray", mathtext_asarray),
    FigureCase::new("gallery_text/mathtext_demo", mathtext_demo),
    FigureCase::new("gallery_text/mathtext_examples", mathtext_examples).sized(7.0, 7.0, 100.0),
    FigureCase::new(
        "gallery_text/mathtext_fontfamily_example",
        mathtext_fontfamily_example,
    )
    .sized(6.0, 5.0, 100.0),
    FigureCase::new("gallery_text/stix_fonts_demo", stix_fonts_demo).sized(8.0, 11.0, 100.0),
];
