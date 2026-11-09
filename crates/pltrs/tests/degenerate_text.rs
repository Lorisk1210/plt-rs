//! Laying out and drawing odd text never panics: broken mathtext, control
//! and bidirectional characters, emoji and other glyphs no bundled font
//! has, very long and empty strings go through every place text appears,
//! in every mathtext fontset, wrapped and rotated, and the figure is
//! rendered to every output format.

use std::panic::{AssertUnwindSafe, catch_unwind};

use pltrs::figure::{SaveFormat, SaveOptions};
use pltrs::legend::LegendOptions;
use pltrs::lines::LineProps;
use pltrs::prelude::*;
use pltrs::text::AnnotateOptions;
use pltrs::transforms::Point;

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
                pltrs::rc::context(|| {
                    pltrs::rc::set("mathtext.fontset", fontset).expect("a fontset");
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

#[test]
fn texts_at_resolutions_of_no_pixel_fail_as_freetype_refuses_them() {
    use pltrs::artist::DrawErrorKind;
    use pltrs::figure::{SaveError, SaveOptions};
    let fig = Figure::with_size(6.4, 4.8, 2.5);
    fig.subplot();
    let Err(SaveError::Draw(errors)) = fig.to_png() else {
        panic!("FreeType refuses fonts of no pixel per em");
    };
    let first = &errors.0[0];
    assert_eq!(first.kind, DrawErrorKind::Library);
    assert_eq!(
        first.message,
        "FT_Set_Char_Size (ft2font.cpp line 248) failed with error 0x97: invalid ppem value"
    );
    // Saved at a resolution of some pixels per em, the texts draw.
    let options = SaveOptions {
        dpi: Some(72.0),
        ..SaveOptions::default()
    };
    assert!(fig.to_png_with(options).is_ok());
}
