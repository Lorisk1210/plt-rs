//! Math text against plain text, the dollar signs that make math text,
//! and text aligned around a box (`mathtext` and `text_props`).

use plt::patches::Rectangle;
use plt::prelude::*;
use plt::text::{HAlign, VAlign};
use plt_core::Rgba;

use crate::figures::{CaseResult, FigureCase};

/// `plt.figure(linewidth=1, edgecolor='black')`: a figure framed in black.
fn framed(fig: &Figure) {
    fig.set_linewidth(1.0);
    fig.set_edgecolor(Rgba::BLACK);
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn mathtext_1(fig: &Figure) -> CaseResult {
    framed(fig);
    fig.text(0.2, 0.7, "plain text: alpha > beta");
    fig.text(0.2, 0.5, "Mathtext: $\\alpha > \\beta$");
    fig.text(0.2, 0.3, r"raw string Mathtext: $\alpha > \beta$");
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn mathtext_2(fig: &Figure) -> CaseResult {
    framed(fig);
    fig.suptitle("Number of unescaped $");
    fig.text(0.1, 0.7, r"odd: $ \alpha $ = $1");
    fig.text(0.1, 0.5, r"even: $ \beta $= $ 2 $");
    fig.text(0.1, 0.3, r"odd: $ \gamma $= \$3 $");
    fig.text(0.1, 0.1, r"even: $ \delta $ = $ \$4 $");
    Ok(())
}

/// The edges of the box `text_props` aligns its texts on, in axes
/// fractions, and its middle.
const LEFT: f64 = 0.25;
const RIGHT: f64 = 0.75;
const BOTTOM: f64 = 0.25;
const TOP: f64 = 0.75;
const MIDDLE: f64 = 0.5;

/// How `text_props` styles a text beyond its alignment.
#[derive(Clone, Copy)]
enum Look {
    /// The default text.
    Plain,
    /// Turned by this many degrees.
    Rotated(f64),
    /// 20 points, red.
    BigRed,
}

/// One text of `text_props`: its position, alignment and look.
struct Placed {
    at: (f64, f64),
    text: &'static str,
    align: (HAlign, VAlign),
    look: Look,
}

/// The texts of `text_props`, in its order.
const PLACED: [Placed; 10] = [
    Placed {
        at: (LEFT, BOTTOM),
        text: "left top",
        align: (HAlign::Left, VAlign::Top),
        look: Look::Plain,
    },
    Placed {
        at: (LEFT, BOTTOM),
        text: "left bottom",
        align: (HAlign::Left, VAlign::Bottom),
        look: Look::Plain,
    },
    Placed {
        at: (RIGHT, TOP),
        text: "right bottom",
        align: (HAlign::Right, VAlign::Bottom),
        look: Look::Plain,
    },
    Placed {
        at: (RIGHT, TOP),
        text: "right top",
        align: (HAlign::Right, VAlign::Top),
        look: Look::Plain,
    },
    Placed {
        at: (RIGHT, BOTTOM),
        text: "center top",
        align: (HAlign::Center, VAlign::Top),
        look: Look::Plain,
    },
    Placed {
        at: (LEFT, MIDDLE),
        text: "right center",
        align: (HAlign::Right, VAlign::Center),
        look: Look::Rotated(90.0),
    },
    Placed {
        at: (LEFT, MIDDLE),
        text: "left center",
        align: (HAlign::Left, VAlign::Center),
        look: Look::Rotated(90.0),
    },
    Placed {
        at: (MIDDLE, MIDDLE),
        text: "middle",
        align: (HAlign::Center, VAlign::Center),
        look: Look::BigRed,
    },
    Placed {
        at: (RIGHT, MIDDLE),
        text: "centered",
        align: (HAlign::Center, VAlign::Center),
        look: Look::Rotated(90.0),
    },
    Placed {
        at: (LEFT, TOP),
        text: "rotated\nwith newlines",
        align: (HAlign::Center, VAlign::Center),
        look: Look::Rotated(45.0),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn text_props_1(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.add_patch_with(
        Rectangle::new((LEFT, BOTTOM), RIGHT - LEFT, TOP - BOTTOM),
        PatchProps::default()
            .fill(false)
            .transform(ax.trans_axes())
            .clip_on(false),
    );
    for placed in PLACED {
        let text = ax.text(placed.at.0, placed.at.1, placed.text);
        text.set_halign(placed.align.0)
            .set_valign(placed.align.1)
            .set_transform(ax.trans_axes());
        match placed.look {
            Look::Plain => {}
            Look::Rotated(angle) => {
                text.set_rotation(angle);
            }
            Look::BigRed => {
                text.set_fontsize(20.0).set_color("red");
            }
        }
    }
    ax.set_axis_off();
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/mathtext_1", mathtext_1).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/mathtext_2", mathtext_2).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_props_1", text_props_1),
];
