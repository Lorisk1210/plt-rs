//! `figures/cases_selectors.py`: cursors and selectors driven by
//! synthetic mouse and key events.

use pltrs::events::{Event, KeyEvent, Location, MouseEvent};
use pltrs::markers::MarkerSymbol;
use pltrs::navigation::PointerButton;
use pltrs::patches::PatchProps;
use pltrs::prelude::*;
use pltrs::widgets::{
    Cursor, CursorOptions, EllipseSelector, Lasso, LassoOptions, LassoSelector,
    LassoSelectorOptions, MultiCursor, Orientation, PolygonSelector, PolygonSelectorOptions,
    RectangleSelector, RectangleSelectorOptions, SelectorUnits, SpanSelector, SpanSelectorOptions,
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
    ax.set_xlim(0.0, 10.0)?.set_ylim(-1.5, 1.5)?;
    Ok(())
}

/// A mouse event at data point `xy` of `ax` with `button` involved.
fn mouse(
    ax: &Axes,
    xy: (f64, f64),
    button: Option<PointerButton>,
) -> Result<MouseEvent, Box<dyn std::error::Error>> {
    Ok(MouseEvent {
        location: Location::from_ax_coords(ax, xy)?,
        button,
        key: None,
        step: 0.0,
        dblclick: false,
    })
}

/// A motion event at data point `xy` of `ax`, with `button` held.
fn motion(ax: &Axes, xy: (f64, f64), button: Option<PointerButton>) -> CaseResult {
    ax.figure()
        .process_event(&Event::Motion(mouse(ax, xy, button)?));
    Ok(())
}

/// A motion event at data point `xy` of `ax`.
fn move_to(ax: &Axes, xy: (f64, f64)) -> CaseResult {
    motion(ax, xy, None)
}

/// A press of `button` at data point `xy` of `ax`.
fn press(ax: &Axes, xy: (f64, f64), button: PointerButton) -> CaseResult {
    ax.figure()
        .process_event(&Event::ButtonPress(mouse(ax, xy, Some(button))?));
    Ok(())
}

/// A release of `button` at data point `xy` of `ax`.
fn release(ax: &Axes, xy: (f64, f64), button: PointerButton) -> CaseResult {
    ax.figure()
        .process_event(&Event::ButtonRelease(mouse(ax, xy, Some(button))?));
    Ok(())
}

/// A key event for `key` at data point `xy` of `ax`.
fn key_event(ax: &Axes, xy: (f64, f64), key: &str) -> Result<KeyEvent, Box<dyn std::error::Error>> {
    Ok(KeyEvent {
        location: Location::from_ax_coords(ax, xy)?,
        key: key.to_owned(),
    })
}

/// matplotlib's `testing.widgets.click_and_drag`: with `key` held, press
/// the left button at data point `start`, move to `end` and release.
fn click_and_drag(ax: &Axes, start: (f64, f64), end: (f64, f64), key: Option<&str>) -> CaseResult {
    let fig = ax.figure();
    if let Some(key) = key {
        fig.process_event(&Event::KeyPress(key_event(ax, start, key)?));
    }
    press(ax, start, PointerButton::Left)?;
    motion(ax, end, Some(PointerButton::Left))?;
    release(ax, end, PointerButton::Left)?;
    if let Some(key) = key {
        fig.process_event(&Event::KeyRelease(key_event(ax, end, key)?));
    }
    Ok(())
}

/// matplotlib's test helper `polygon_place_vertex`: move to data point
/// `xy` and click there.
fn place_vertex(ax: &Axes, xy: (f64, f64)) -> CaseResult {
    move_to(ax, xy)?;
    press(ax, xy, PointerButton::Left)?;
    release(ax, xy, PointerButton::Left)
}

/// `LOOP`: twelve points along an elliptical arc.
fn lasso_loop() -> Vec<(f64, f64)> {
    linspace(0.0, 5.0, 12)
        .into_iter()
        .map(|a| (2.0 + 1.5 * a.cos(), 1.0 * a.sin()))
        .collect()
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
        let _cursor = Cursor::new(&ax, options);
        move_to(&ax, (4.0, 0.5))
    }),
    FigureCase::new("selectors/multicursor", |fig| {
        let axs = fig.subplots(2, 1)?;
        let (top, bottom) = (&axs[(0, 0)], &axs[(1, 0)]);
        plot(top)?;
        let t = t();
        let y: Vec<f64> = t.iter().map(|t| t.cos()).collect();
        bottom.plot_with(t, y, LineProps::fmt("g")?)?;
        bottom.set_xlim(0.0, 10.0)?.set_ylim(-1.5, 1.5)?;
        let options = CursorOptions {
            horizontal: true,
            useblit: false,
            line: LineProps::default().color("r").linewidth(1.0),
            ..CursorOptions::multicursor()
        };
        let _multi = MultiCursor::new(&[top.clone(), bottom.clone()], options);
        move_to(bottom, (6.0, 0.25))
    }),
    FigureCase::new("selectors/span_interactive", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = SpanSelectorOptions {
            props: PatchProps::default().facecolor("tab:blue").alpha(0.3),
            ..interactive()
        };
        let _span = SpanSelector::new(&ax, Orientation::Horizontal, options);
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
        let _span = SpanSelector::new(&ax, Orientation::Vertical, options);
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
        let span = SpanSelector::new(&ax, Orientation::Horizontal, options);
        span.set_extents((3.0, 5.0));
        click_and_drag(&ax, (4.0, 0.0), (6.0, 0.0), None)
    }),
    FigureCase::new("selectors/span_move_key", |fig| {
        let axs = fig.subplots(1, 2)?;
        let (left, right) = (&axs[(0, 0)], &axs[(0, 1)]);
        plot(left)?;
        plot(right)?;
        let _moved = SpanSelector::new(left, Orientation::Horizontal, interactive());
        click_and_drag(left, (2.0, 0.0), (4.0, 0.0), None)?;
        click_and_drag(left, (8.0, 0.0), (9.0, 0.0), Some(" "))?;
        let options = SpanSelectorOptions {
            minspan: 1.0,
            ..interactive()
        };
        let _removed = SpanSelector::new(right, Orientation::Horizontal, options);
        click_and_drag(right, (2.0, 0.0), (6.0, 0.0), None)?;
        click_and_drag(right, (8.0, 0.0), (8.5, 0.0), None)
    }),
    FigureCase::new("selectors/rectangle_interactive", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let _rect = RectangleSelector::new(&ax, interactive_region());
        click_and_drag(&ax, (2.0, -1.0), (6.0, 1.0), None)?;
        click_and_drag(&ax, (6.0, 1.0), (8.0, 1.2), None)
    }),
    FigureCase::new("selectors/rectangle_square_center", |fig| {
        let (left, right) = pair(fig)?;
        let square = RectangleSelectorOptions {
            props: PatchProps::default().facecolor("tab:green").alpha(0.3),
            ..interactive_region()
        };
        let _square = RectangleSelector::new(&left, square);
        click_and_drag(&left, (2.0, -1.0), (6.0, 0.0), Some("shift"))?;
        let centred = RectangleSelectorOptions {
            handle_props: LineProps::default()
                .markeredgecolor(pltrs::rc::ColorOrAuto::Color("r".into()))
                .markersize(9.0),
            ..interactive_region()
        };
        let _centred = RectangleSelector::new(&right, centred);
        click_and_drag(&right, (5.0, 0.0), (7.0, 0.8), Some("control"))
    }),
    FigureCase::new("selectors/rectangle_rotated", |fig| {
        let (left, right) = pair(fig)?;
        let set_angle = RectangleSelector::new(&left, interactive_region());
        set_angle.set_extents((3.0, 7.0, -0.5, 0.5));
        set_angle.set_rotation(25.0);
        let _dragged = RectangleSelector::new(&right, interactive_region());
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
        let _rect = RectangleSelector::new(&ax, options);
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
        let _ellipse = EllipseSelector::new(&left, blue);
        click_and_drag(&left, (2.0, -1.0), (8.0, 1.0), None)?;
        let pixels = RectangleSelectorOptions {
            spancoords: SelectorUnits::Pixels,
            minspanx: 30.0,
            minspany: 30.0,
            ..interactive_region()
        };
        let plain = EllipseSelector::new(&right, pixels);
        click_and_drag(&right, (2.0, -1.0), (7.0, 0.5), None)?;
        plain.set_rotation(-30.0);
        Ok(())
    }),
    FigureCase::new("selectors/lasso", |fig| {
        let (left, right) = pair(fig)?;
        let options = LassoSelectorOptions {
            props: LineProps::default().color("m").linewidth(2.0),
            useblit: false,
            ..LassoSelectorOptions::default()
        };
        let _selector = LassoSelector::new(&left, options);
        let path = lasso_loop();
        press(&left, path[0], PointerButton::Left)?;
        for &xy in &path[1..] {
            motion(&left, xy, Some(PointerButton::Left))?;
        }
        let options = LassoOptions {
            props: LineProps::default().color("tab:red"),
            useblit: false,
        };
        let _lasso = Lasso::new(&right, (5.0, 0.0), options);
        for xy in [(6.0, 0.5), (7.0, 0.2), (6.5, -0.6), (5.5, -0.4)] {
            motion(&right, xy, Some(PointerButton::Left))?;
        }
        Ok(())
    }),
    FigureCase::new("selectors/polygon_complete", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let _polygon = PolygonSelector::new(&ax, PolygonSelectorOptions::default());
        for xy in [
            (2.0, -1.0),
            (6.0, -1.2),
            (7.0, 0.5),
            (4.0, 1.2),
            (2.0, -1.0),
        ] {
            place_vertex(&ax, xy)?;
        }
        click_and_drag(&ax, (7.0, 0.5), (8.0, 0.8), None)
    }),
    FigureCase::new("selectors/polygon_open", |fig| {
        let (left, right) = pair(fig)?;
        let options = PolygonSelectorOptions {
            props: LineProps::default().color("tab:green").linewidth(3.0),
            handle_props: Some(
                LineProps::default()
                    .marker(MarkerSymbol::Square)
                    .markersize(5.0),
            ),
            ..PolygonSelectorOptions::default()
        };
        let _polygon = PolygonSelector::new(&left, options);
        for xy in [(2.0, -1.0), (6.0, -1.0), (7.0, 0.5)] {
            place_vertex(&left, xy)?;
        }
        move_to(&left, (3.0, 1.0))?;
        let moved = PolygonSelector::new(&right, PolygonSelectorOptions::default());
        moved.set_verts(&[(2.0, -1.0), (5.0, -1.0), (3.5, 1.0)])?;
        click_and_drag(&right, (3.0, 0.0), (5.0, 0.2), Some("shift"))
    }),
    FigureCase::new("selectors/polygon_box", |fig| {
        let ax = fig.subplot();
        plot(&ax)?;
        let options = PolygonSelectorOptions {
            draw_bounding_box: true,
            box_props: Some(
                PatchProps::default()
                    .facecolor("none")
                    .edgecolor("tab:orange"),
            ),
            ..PolygonSelectorOptions::default()
        };
        let _polygon = PolygonSelector::new(&ax, options);
        for xy in [(2.0, -1.0), (6.0, -1.0), (4.0, 1.0), (2.0, -1.0)] {
            place_vertex(&ax, xy)?;
        }
        click_and_drag(&ax, (6.0, 1.0), (8.0, 1.3), None)
    }),
];
