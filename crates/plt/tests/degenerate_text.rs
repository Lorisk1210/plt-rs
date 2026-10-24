//! Laying out and drawing odd text never panics: broken mathtext, control
//! and bidirectional characters, emoji and other glyphs no bundled font
//! has, very long and empty strings go through every place text appears,
//! in every mathtext fontset, wrapped and rotated, and the figure is
//! rendered to every output format.

use std::panic::{AssertUnwindSafe, catch_unwind};

use plt::figure::{SaveFormat, SaveOptions};
use plt::legend::LegendOptions;
use plt::lines::LineProps;
use plt::prelude::*;
use plt::text::AnnotateOptions;
use plt::transforms::Point;

/// The strings every case is drawn with.
fn strings() -> Vec<String> {
    [
        "",
        " ",
        "$",
        "$$",
        "$x",
        r"$\frac{1}{$",
        r"$x^{2_{3}$",
        r"$\sqrt[3]{}$",
        r"$\unknowncommand{x}$",
        r"$\left( x \right.$",
        r"$\mathcal{ABC}\,\mathbb{R}\,\mathfrak{g}$",
        r"$a^{b^{c^{d^{e^{f}}}}}$",
        r"\$5 and \$6",
        "a\nb\n\n\nc",
        "tab\there",
        "nul\0char",
        "\u{202e}reversed\u{202c} text",
        "שלום עולם",
        "مرحبا بالعالم",
        "emoji 😀🎉",
        "combining e\u{301}\u{302}\u{303}",
        "\u{fffd}\u{ffff}\u{10ffff}",
        "日本語のテキスト",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(["word ".repeat(400), format!("${}$", r"\alpha".repeat(200))])
    .collect()
}

/// Puts `text` everywhere text appears on a figure.
fn decorate(fig: &Figure, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    ax.plot_with(
        vec![0.0, 1.0],
        vec![0.0, 1.0],
        LineProps::default().label(text),
    )?;
    ax.set_title(text);
    ax.set_xlabel(text).set_ylabel(text);
    ax.set_xticks([0.0, 0.5, 1.0]);
    ax.set_xticklabels([text, text, text])?;
    ax.text(0.5, 0.5, text).set_rotation(33.0).set_wrap(true);
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.2, 0.8).into()),
        ..AnnotateOptions::default()
    };
    ax.annotate(text, Point::new(0.5, 0.5), options)?;
    ax.legend(LegendOptions::default())?;
    fig.suptitle(text);
    fig.text(0.1, 0.1, text);
    Ok(())
}

#[test]
fn odd_text_never_panics() {
    let mut panics = Vec::new();
    for fontset in ["dejavusans", "dejavuserif", "cm", "stix", "stixsans"] {
        for text in strings() {
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                plt::rc::context(|| {
                    plt::rc::set("mathtext.fontset", fontset).expect("a fontset");
                    let fig = Figure::with_size(3.0, 2.0, 40.0);
                    // Errors are allowed: matplotlib rejects broken mathtext too.
                    let _ = decorate(&fig, &text);
                    for format in [
                        SaveFormat::Png,
                        SaveFormat::Svg,
                        SaveFormat::Pdf,
                        SaveFormat::Eps,
                    ] {
                        let _ = fig.to_format(format, SaveOptions::default());
                    }
                });
            }));
            if outcome.is_err() {
                let shown: String = text.chars().take(40).collect();
                panics.push(format!("{shown:?} in {fontset}"));
            }
        }
    }
    assert!(panics.is_empty(), "panicked: {panics:#?}");
    // Plain text goes everywhere, so the cases above reach every place.
    let plain = decorate(&Figure::new(), "plain $x^2$").map_err(|err| err.to_string());
    assert_eq!(plain, Ok(()));
}
