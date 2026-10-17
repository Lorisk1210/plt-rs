//! `figures/cases_selectors.py`: cursors and selectors driven by
//! synthetic mouse and key events.

use plt::events::EventSource;
use plt::navigation::PointerButton;
use plt::patches::PatchProps;
use plt::prelude::*;
use plt::transforms::Point;
use plt::widgets::{
    Cursor, CursorOptions, EllipseSelector, MultiCursor, Orientation, RectangleSelector,
    RectangleSelectorOptions, SelectorUnits, SpanSelector, SpanSelectorOptions,
};

use crate::figures::{CaseResult, FigureCase};

/// `T = np.linspace(0, 10, 60)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 60)
}

/// The sine over `T` with the view fixed to `[0, 10] × [-1.5, 1.5]`.
fn plot(ax: &Axes) -> CaseResult {
    let t = t();
    let y: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    ax.plot(t, y)?;
    ax.set_xlim(0.0, 10.0).set_ylim(-1.5, 1.5);
    Ok(())
}

/// Display coordinates of data point `(x, y)` of `ax`.
fn display(ax: &Axes, (x, y): (f64, f64)) -> Result<Point, Box<dyn std::error::Error>> {
    let trans = ax.figure().resolve_transform(&ax.trans_data())?;
    Ok(trans.transform_point(Point::new(x, y)))
}

/// A motion event at data point `xy` of `ax`.
fn move_to(ax: &Axes, xy: (f64, f64)) -> CaseResult {
    let at = display(ax, xy)?;
    EventSource::new().motion(&ax.figure(), at.x, at.y);
    Ok(())
}

/// matplotlib's `testing.widgets.click_and_drag`: with `key` held, press
/// the left button at data point `start`, move to `end` and release.
fn click_and_drag(ax: &Axes, start: (f64, f64), end: (f64, f64), key: Option<&str>) -> CaseResult {
    let fig = ax.figure();
    let (a, b) = (display(ax, start)?, display(ax, end)?);
    let mut source = EventSource::new();
    if let Some(key) = key {
        source.key_press(&fig, key, a.x, a.y);
    }
    source.button_press(&fig, a.x, a.y, PointerButton::Left, false);
    source.motion(&fig, b.x, b.y);
    source.button_release(&fig, b.x, b.y, PointerButton::Left);
    if let Some(key) = key {
        source.key_release(&fig, key, b.x, b.y);
    }
    Ok(())
}

/// Interactive rectangle options.
fn interactive_region() -> RectangleSelectorOptions {
    RectangleSelectorOptions {
        interactive: true,
        ..RectangleSelectorOptions::default()
    }
}

/// Two axes side by side, both showing the sine.
fn pair(fig: &Figure) -> Result<(Axes, Axes), Box<dyn std::error::Error>> {
    let axs = fig.subplots(1, 2)?;
    let (left, right) = (axs[(0, 0)].clone(), axs[(0, 1)].clone());
    plot(&left)?;
    plot(&right)?;
    Ok((left, right))
}

/// Interactive span options.
fn interactive() -> SpanSelectorOptions {
    SpanSelectorOptions {
        interactive: true,
        ..SpanSelectorOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("selectors/cursor", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = CursorOptions {
            line: LineProps::default().color("red").linewidth(1.0),
            ..CursorOptions::default()
        };
        let _cursor = Cursor::new(&ax, &options);
        move_to(&ax, (4.0, 0.5))
    }),
    FigureCase::new("selectors/multicursor", |fig| {
        let axs = fig.subplots(2, 1)?;
        let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
        plot(top)?;
        let t = t();
        let y: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        bottom.plot_with(t, y, LineProps::fmt("g")?)?;
        bottom.set_xlim(0.0, 10.0).set_ylim(-1.5, 1.5);
        let options = CursorOptions {
            horizontal: true,
            line: LineProps::default().color("r").linewidth(1.0),
            ..CursorOptions::multicursor()
        };
        let _multi = MultiCursor::new(&[top.clone(), bottom.clone()], &options);
        move_to(bottom, (6.0, 0.25))
    }),
    FigureCase::new("selectors/span_interactive", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = SpanSelectorOptions {
            props: PatchProps::default().facecolor("tab:blue").alpha(0.3),
            ..interactive()
        };
        let _span = SpanSelector::new(&ax, Orientation::Horizontal, &options);
        click_and_drag(&ax, (2.0, 0.0), (6.0, 0.0), None)?;
        click_and_drag(&ax, (6.0, 0.5), (7.5, 0.5), None)
    }),
    FigureCase::new("selectors/span_vertical_snap", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = SpanSelectorOptions {
            snap_values: Some(linspace(-1.5, 1.5, 13)),
            handle_props: LineProps::default().color("k").linewidth(2.0),
            ..interactive()
        };
        let _span = SpanSelector::new(&ax, Orientation::Vertical, &options);
        click_and_drag(&ax, (5.0, -0.9), (5.0, 0.8), None)
    }),
    FigureCase::new("selectors/span_drag_from_anywhere", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = SpanSelectorOptions {
            drag_from_anywhere: true,
            props: PatchProps::default().facecolor("orange").alpha(0.4),
            ..interactive()
        };
        let span = SpanSelector::new(&ax, Orientation::Horizontal, &options);
        span.set_extents((3.0, 5.0));
        click_and_drag(&ax, (4.0, 0.0), (6.0, 0.0), None)
    }),
    FigureCase::new("selectors/span_move_key", |fig| {
        let axs = fig.subplots(1, 2)?;
        let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
        plot(left)?;
        plot(right)?;
        let _moved = SpanSelector::new(left, Orientation::Horizontal, &interactive());
        click_and_drag(left, (2.0, 0.0), (4.0, 0.0), None)?;
        click_and_drag(left, (8.0, 0.0), (9.0, 0.0), Some(" "))?;
        let options = SpanSelectorOptions {
            minspan: 1.0,
            ..interactive()
        };
        let _removed = SpanSelector::new(right, Orientation::Horizontal, &options);
        click_and_drag(right, (2.0, 0.0), (6.0, 0.0), None)?;
        click_and_drag(right, (8.0, 0.0), (8.5, 0.0), None)
    }),
    FigureCase::new("selectors/rectangle_interactive", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let _rect = RectangleSelector::new(&ax, &interactive_region());
        click_and_drag(&ax, (2.0, -1.0), (6.0, 1.0), None)?;
        click_and_drag(&ax, (6.0, 1.0), (8.0, 1.2), None)
    }),
    FigureCase::new("selectors/rectangle_square_center", |fig| {
        let (left, right) = pair(fig)?;
        let square = RectangleSelectorOptions {
            props: PatchProps::default().facecolor("tab:green").alpha(0.3),
            ..interactive_region()
        };
        let _square = RectangleSelector::new(&left, &square);
        click_and_drag(&left, (2.0, -1.0), (6.0, 0.0), Some("shift"))?;
        let centred = RectangleSelectorOptions {
            handle_props: LineProps::default()
                .markeredgecolor(plt::rc::ColorOrAuto::Color("r".into()))
                .markersize(9.0),
            ..interactive_region()
        };
        let _centred = RectangleSelector::new(&right, &centred);
        click_and_drag(&right, (5.0, 0.0), (7.0, 0.8), Some("control"))
    }),
    FigureCase::new("selectors/rectangle_rotated", |fig| {
        let (left, right) = pair(fig)?;
        let set_angle = RectangleSelector::new(&left, &interactive_region());
        set_angle.set_extents((3.0, 7.0, -0.5, 0.5));
        set_angle.set_rotation(25.0);
        let _dragged = RectangleSelector::new(&right, &interactive_region());
        click_and_drag(&right, (3.0, -0.5), (7.0, 0.5), None)?;
        click_and_drag(&right, (7.0, -0.5), (7.0, 0.2), Some("r"))
    }),
    FigureCase::new("selectors/rectangle_move_key", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = RectangleSelectorOptions {
            drag_from_anywhere: true,
            ..interactive_region()
        };
        let _rect = RectangleSelector::new(&ax, &options);
        click_and_drag(&ax, (1.0, -1.0), (3.0, 0.0), None)?;
        click_and_drag(&ax, (8.0, 1.0), (9.0, 1.2), Some(" "))?;
        click_and_drag(&ax, (3.0, -0.3), (4.0, -0.5), None)
    }),
    FigureCase::new("selectors/ellipse", |fig| {
        let (left, right) = pair(fig)?;
        let blue = RectangleSelectorOptions {
            props: PatchProps::default()
                .facecolor("tab:blue")
                .edgecolor("k")
                .alpha(0.3),
            ..interactive_region()
        };
        let _ellipse = EllipseSelector::new(&left, &blue);
        click_and_drag(&left, (2.0, -1.0), (8.0, 1.0), None)?;
        let pixels = RectangleSelectorOptions {
            spancoords: SelectorUnits::Pixels,
            minspanx: 30.0,
            minspany: 30.0,
            ..interactive_region()
        };
        let plain = EllipseSelector::new(&right, &pixels);
        click_and_drag(&right, (2.0, -1.0), (7.0, 0.5), None)?;
        plain.set_rotation(-30.0);
        Ok(())
    }),
];
