//! matplotlib's text tutorials ("Text in Matplotlib", "Text properties
//! and layout", "Writing mathematical expressions") in pltrs (Rust):
//! titles and labels, text placed in data and axes coordinates, fonts,
//! alignment, rotation, boxes, and mathtext.
//!
//! ```text
//! cargo run -p pltrs-examples --example text_and_mathtext   # writes text_*.png here
//! ```

use pltrs::fonts::{FontStyle, FontWeight, Fontset};
use pltrs::layout::LayoutEngine;
use pltrs::patches::{BoxStyle, Rectangle};
use pltrs::prelude::*;
use pltrs::text::{HAlign, TextBoxProps, VAlign};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn main() -> Outcome {
    basic_text()?;
    alignment()?;
    mathtext()?;
    Ok(())
}

/// "Text in Matplotlib": `fig.suptitle`, `ax.set_title`,
/// `ax.set_xlabel`, `ax.text(x, y, s, style=..., bbox=...)`,
/// `ax.text(..., transform=ax.transAxes)` and `fig.text`.
fn basic_text() -> Outcome {
    let fig = Figure::with_size(6.0, 4.0, 100.0);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    fig.suptitle("bold figure suptitle")
        .set_fontsize(14.0)
        .set_weight(FontWeight::BOLD);
    let ax = fig.subplot();
    ax.set_title("axes title");
    ax.set_xlabel("xlabel").set_ylabel("ylabel");
    ax.set_xlim(0.0, 10.0)?.set_ylim(0.0, 10.0)?;

    let boxed = ax.text(3.0, 8.0, "boxed italics text in data coords");
    boxed.set_style(FontStyle::Italic);
    boxed.set_bbox(Some(
        TextBoxProps::new("square,pad=0.5".parse::<BoxStyle>()?)
            .with_patch(PatchProps::default().facecolor("red").alpha(0.5)),
    ));
    ax.text(2.0, 6.0, r"an equation: $E=mc^2$")
        .set_fontsize(15.0);
    ax.text(3.0, 2.0, "Unicode: Institut für Festkörperphysik");
    let corner = ax.text(0.95, 0.01, "colored text in axes coords");
    corner
        .set_transform(ax.trans_axes())
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Bottom)
        .set_color("green")
        .set_fontsize(15.0);
    ax.plot_fmt([2.0], [1.0], "o")?;
    fig.text(0.01, 0.01, "figure text").set_color("0.5");
    fig.savefig("text_basic.png")?;
    Ok(())
}

/// "Text properties and layout": `horizontalalignment`,
/// `verticalalignment`, `rotation`, `family`, `weight` and `size`
/// around one rectangle.
fn alignment() -> Outcome {
    let fig = Figure::with_size(6.0, 4.0, 100.0);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    ax.set_axis_off();
    let (left, bottom, width, height) = (0.25, 0.25, 0.5, 0.5);
    let (right, top) = (left + width, bottom + height);
    let frame = ax.add_patch_with(
        Rectangle::new((left, bottom), width, height),
        PatchProps::default().fill(false),
    );
    frame.set_transform(ax.trans_axes()).set_clip_on(false);

    let place = |x: f64, y: f64, s: &str, halign, valign, rotation: f64| {
        let text = ax.text(x, y, s);
        text.set_transform(ax.trans_axes())
            .set_halign(halign)
            .set_valign(valign)
            .set_rotation(rotation);
        text
    };
    place(left, bottom, "left top", HAlign::Left, VAlign::Top, 0.0);
    place(
        left,
        bottom,
        "left bottom",
        HAlign::Left,
        VAlign::Bottom,
        0.0,
    );
    place(
        right,
        top,
        "right bottom",
        HAlign::Right,
        VAlign::Bottom,
        0.0,
    );
    place(right, top, "right top", HAlign::Right, VAlign::Top, 0.0);
    place(
        right,
        bottom,
        "center top",
        HAlign::Center,
        VAlign::Top,
        0.0,
    );
    place(
        left,
        f64::midpoint(bottom, top),
        "right center",
        HAlign::Right,
        VAlign::Center,
        90.0,
    );
    place(
        left,
        f64::midpoint(bottom, top),
        "left center",
        HAlign::Left,
        VAlign::Center,
        90.0,
    );
    place(0.5, 0.5, "middle", HAlign::Center, VAlign::Center, 0.0)
        .set_fontsize(20.0)
        .set_color("red");
    place(
        right,
        f64::midpoint(bottom, top),
        "centered",
        HAlign::Center,
        VAlign::Center,
        90.0,
    );
    place(
        left,
        top,
        "rotated\nwith newlines",
        HAlign::Left,
        VAlign::Top,
        45.0,
    );
    place(
        0.5,
        0.1,
        "a serif family, bold",
        HAlign::Center,
        VAlign::Center,
        0.0,
    )
    .set_family(["DejaVu Serif"])
    .set_weight(FontWeight::BOLD);
    fig.savefig("text_alignment.png")?;
    Ok(())
}

/// "Writing mathematical expressions": sub- and superscripts, fractions,
/// radicals, accents, Greek letters, fonts within math (`\mathrm`,
/// `\mathcal`), and `mathtext.fontset`.
fn mathtext() -> Outcome {
    let expressions = [
        r"$\alpha_i > \beta_i$",
        r"$\sum_{i=0}^\infty x_i$",
        r"$\frac{3}{4} \binom{3}{4} \genfrac{}{}{0}{}{3}{4}$",
        r"$\sqrt{2}\ \sqrt[3]{x}$",
        r"$\hat a\ \bar b\ \vec c\ \dot d\ \ddot e\ \tilde f$",
        r"$\mathrm{Roman}\ \mathit{Italic}\ \mathcal{CALLIGRAPHY}\ \mathbb{BLACKBOARD}$",
        r"$W^{3\beta}_{\delta_1 \rho_1 \sigma_2} = U^{3\beta}_{\delta_1 \rho_1} + \frac{1}{8 \pi 2}$",
    ];
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    ax.set_axis_off();
    let rows = expressions.len() + 3;
    for (row, expression) in expressions.iter().enumerate() {
        let y = 1.0 - (row as f64 + 0.7) / rows as f64;
        ax.text(0.04, y, *expression)
            .set_transform(ax.trans_axes())
            .set_fontsize(15.0);
    }
    for (row, fontset) in [Fontset::Cm, Fontset::Stix, Fontset::DejaVuSerif]
        .into_iter()
        .enumerate()
    {
        let y = 1.0 - ((expressions.len() + row) as f64 + 0.7) / rows as f64;
        let text = ax.text(
            0.04,
            y,
            format!(r"$\int_0^\infty e^{{-x^2}}\,dx = \frac{{\sqrt{{\pi}}}}{{2}}$  ({fontset:?})"),
        );
        text.set_transform(ax.trans_axes())
            .set_fontsize(15.0)
            .set_math_fontfamily(fontset);
    }
    fig.savefig("text_mathtext.png")?;
    Ok(())
}
