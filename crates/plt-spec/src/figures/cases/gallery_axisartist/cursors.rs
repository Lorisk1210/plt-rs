//! Cursors: `annotated_cursor`, `cursor`, `mouse_cursor` and
//! `multicursor`, drawn before the pointer moves.

use plt::events::CursorShape;
use plt::figure::Share;
use plt::fonts::FontWeight;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::prelude::*;
use plt::text::{HAlign, VAlign};
use plt::widgets::{Cursor, CursorOptions, MultiCursor};

use super::{sine, t};
use crate::figures::cases::common::{map, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_axisartist/annotated_cursor_1", |fig| {
        annotated_cursor(fig, "Cursor Tracking x Position")
    })
    .sized(8.0, 6.0, 100.0),
    FigureCase::new("gallery_axisartist/annotated_cursor_2", |fig| {
        annotated_cursor(fig, "Cursor Tracking y Position")
    })
    .sized(8.0, 6.0, 100.0),
    FigureCase::new("gallery_axisartist/cursor", cursor).sized(8.0, 6.0, 100.0),
    // An inch for each of the seven cursor shapes, and half an inch more.
    FigureCase::new("gallery_axisartist/mouse_cursor", mouse_cursor).sized(6.0, 7.5, 100.0),
    FigureCase::new("gallery_axisartist/multicursor_1", multicursor_1),
    FigureCase::new("gallery_axisartist/multicursor_2", multicursor_2),
];

/// The red blitted crosshair of `annotated_cursor` and `cursor`.
fn red_crosshair() -> CursorOptions {
    CursorOptions {
        useblit: true,
        line: LineProps::default().color("red").linewidth(2.0),
        ..CursorOptions::default()
    }
}

/// The red vertical line `multicursor` draws across its axes.
fn red_multicursor(axes: &[Axes]) -> MultiCursor {
    let options = CursorOptions {
        line: LineProps::default().color("r").linewidth(1.0),
        ..CursorOptions::multicursor()
    };
    MultiCursor::new(axes, options)
}

/// A figure of `annotated_cursor`: a parabola under a red crosshair whose
/// text, blue and bold, stays hidden until the pointer moves.
fn annotated_cursor(fig: &Figure, title: &str) -> CaseResult {
    let ax = fig.subplot();
    ax.set_title(title);
    let x = linspace(-5.0, 5.0, 1000);
    let y = map(&x, |x| x * x);
    ax.plot(x, y)?;
    ax.set_xlim(-5.0, 5.0)?.set_ylim(0.0, 25.0)?;
    let _cursor = Cursor::new(&ax, red_crosshair());
    // At the lower left corner of the view (`get_xbound()[0]`,
    // `get_ybound()[0]`).
    ax.text(-5.0, 0.0, "0, 0")
        .set_color("blue")
        .set_weight(FontWeight::BOLD)
        .set_visible(false);
    Ok(())
}

fn cursor(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let xy = SpecRng::new(19_680_801).uniform(200, 0.0, 1.0);
    let (x, y) = xy.split_at(100);
    ax.plot_fmt(
        map(x, |v| 4.0 * (v - 0.5)),
        map(y, |v| 4.0 * (v - 0.5)),
        "o",
    )?;
    ax.set_xlim(-2.0, 2.0)?.set_ylim(-2.0, 2.0)?;
    let _cursor = Cursor::new(&ax, red_crosshair());
    Ok(())
}

fn mouse_cursor(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        CursorShape::ALL.len(),
        1,
        None,
        None,
        SubplotOverrides {
            hspace: Some(0.0),
            ..SubplotOverrides::default()
        },
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    fig.suptitle("Hover over an Axes to see alternate Cursors");
    for (shape, ax) in CursorShape::ALL.into_iter().zip(axs.as_slice()) {
        ax.text(0.5, 0.5, shape.name())
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center);
        tickless(ax);
    }
    Ok(())
}

fn multicursor_1(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    let t = t();
    ax1.plot(t.clone(), sine(&t, 1.0))?;
    ax2.plot(t.clone(), sine(&t, 1.5))?;
    let _multi = red_multicursor(&[ax1.clone(), ax2.clone()]);
    Ok(())
}

fn multicursor_2(fig: &Figure) -> CaseResult {
    let ax3 = fig.subplot();
    let t = t();
    ax3.plot(t.clone(), sine(&t, 2.0))?;
    let _multi = red_multicursor(&[ax3]);
    Ok(())
}
