//! The toolbar icons, drawn as vector outlines on a 24-unit grid.

use plt_core::{Path, Point};
use plt_ui::Icon;

use super::GuiError;

const GRID: f64 = 24.0;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn line(points: &[(f64, f64)]) -> Path {
    Path::polyline(points.iter().map(|&(x, y)| p(x, y)).collect())
}

/// A house: back to the original view.
pub(super) fn home() -> Icon {
    Icon::new(GRID)
        .stroke(line(&[(3.5, 11.5), (12.0, 4.0), (20.5, 11.5)]))
        .stroke(line(&[(6.0, 9.5), (6.0, 20.0), (18.0, 20.0), (18.0, 9.5)]))
        .stroke(line(&[
            (10.0, 20.0),
            (10.0, 14.0),
            (14.0, 14.0),
            (14.0, 20.0),
        ]))
}

/// An arrow pointing left (`sign = -1`) or right (`sign = 1`).
fn arrow(sign: f64) -> Icon {
    let x = |dx: f64| 12.0 + sign * dx;
    Icon::new(GRID)
        .stroke(line(&[(x(-7.0), 12.0), (x(7.0), 12.0)]))
        .stroke(line(&[(x(1.5), 6.5), (x(7.0), 12.0), (x(1.5), 17.5)]))
}

/// A left arrow: the previous view.
pub(super) fn back() -> Icon {
    arrow(-1.0)
}

/// A right arrow: the next view.
pub(super) fn forward() -> Icon {
    arrow(1.0)
}

/// Four arrows from the centre: pan.
pub(super) fn pan() -> Icon {
    let heads = [
        [(9.5, 6.0), (12.0, 3.5), (14.5, 6.0)],
        [(9.5, 18.0), (12.0, 20.5), (14.5, 18.0)],
        [(6.0, 9.5), (3.5, 12.0), (6.0, 14.5)],
        [(18.0, 9.5), (20.5, 12.0), (18.0, 14.5)],
    ];
    heads.iter().fold(
        Icon::new(GRID)
            .stroke(line(&[(12.0, 3.5), (12.0, 20.5)]))
            .stroke(line(&[(3.5, 12.0), (20.5, 12.0)])),
        |icon, head| icon.stroke(line(head)),
    )
}

/// A magnifying glass: zoom to a rectangle.
pub(super) fn zoom() -> Icon {
    Icon::new(GRID)
        .stroke(Path::circle(p(10.5, 10.5), 6.0))
        .stroke(line(&[(15.0, 15.0), (20.0, 20.0)]))
        .stroke(line(&[(8.0, 10.5), (13.0, 10.5)]))
        .stroke(line(&[(10.5, 8.0), (10.5, 13.0)]))
}

/// Three sliders with their knobs: configure the subplots.
pub(super) fn subplots() -> Icon {
    const KNOB: f64 = 2.5;
    [(6.0, 15.0), (12.0, 8.0), (18.0, 13.0)]
        .iter()
        .fold(Icon::new(GRID), |icon, &(y, knob)| {
            icon.stroke(line(&[(4.0, y), (knob - KNOB, y)]))
                .stroke(line(&[(knob + KNOB, y), (20.0, y)]))
                .stroke(Path::circle(p(knob, y), KNOB))
        })
}

/// A line chart over two axes: the plt-rs window icon.
pub(super) fn app() -> Icon {
    Icon::new(GRID)
        .with_stroke_width(2.2)
        .stroke(line(&[(4.0, 3.5), (4.0, 20.0), (20.5, 20.0)]))
        .stroke(line(&[
            (7.0, 15.5),
            (11.0, 10.0),
            (14.5, 13.0),
            (19.5, 5.5),
        ]))
}

/// The icon of plt-rs windows: [`app`] in white on the theme's accent,
/// 64 pixels wide.
pub(super) fn window_icon() -> Result<plt_platform::Icon, GuiError> {
    const SIZE: u32 = 64;
    let accent = plt_ui::Theme::default().accent;
    let rgba = app().app_icon(SIZE, accent, plt_core::Rgba::WHITE)?;
    Ok(plt_platform::Icon::from_rgba(SIZE, SIZE, rgba)?)
}

/// A folder, for directories in file lists.
pub(super) fn folder() -> Icon {
    Icon::new(GRID).stroke(line(&[
        (3.5, 19.5),
        (3.5, 5.5),
        (9.5, 5.5),
        (11.5, 8.0),
        (20.5, 8.0),
        (20.5, 19.5),
        (3.5, 19.5),
    ]))
}

/// A sheet with a folded corner, for files in file lists.
pub(super) fn file() -> Icon {
    Icon::new(GRID)
        .stroke(line(&[
            (6.0, 3.5),
            (14.0, 3.5),
            (18.5, 8.0),
            (18.5, 20.5),
            (6.0, 20.5),
            (6.0, 3.5),
        ]))
        .stroke(line(&[(14.0, 3.5), (14.0, 8.0), (18.5, 8.0)]))
}

/// A tray with a downward arrow: save the figure.
pub(super) fn save() -> Icon {
    Icon::new(GRID)
        .stroke(line(&[(12.0, 4.0), (12.0, 15.0)]))
        .stroke(line(&[(7.5, 10.5), (12.0, 15.0), (16.5, 10.5)]))
        .stroke(line(&[
            (4.5, 15.5),
            (4.5, 19.5),
            (19.5, 19.5),
            (19.5, 15.5),
        ]))
}
