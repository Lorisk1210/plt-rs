//! The cases from `test_widgets.py`: radio and check buttons in every
//! layout, styled on creation and through their setters, an invisible
//! rectangle selector and polygon selectors given their vertices.

use plt::colors::Color;
use plt::fonts::{FontProperties, FontSize};
use plt::prelude::*;
use plt::text::TextProps;
use plt::widgets::{
    ButtonLayout, ButtonMarkerProps, CheckButtons, CheckButtonsOptions, PolygonSelector,
    PolygonSelectorOptions, RadioButtons, RadioButtonsOptions, RectangleSelector,
    RectangleSelectorOptions,
};

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// The labels of the styled buttons.
const RADIO_LABELS: [&str; 3] = ["Radio 1", "Radio 2", "Radio 3"];
const CHECK_LABELS: [&str; 3] = ["Check 1", "Check 2", "Check 3"];

/// The colours of the styled buttons.
const STYLED_COLORS: [&str; 3] = ["red", "green", "blue"];

/// Colours by name.
fn colors(names: &[&str]) -> Vec<Color> {
    names.iter().copied().map(Color::from).collect()
}

/// Text properties of labels in `color` at `points`.
fn label(color: &str, points: f64) -> TextProps {
    let rc = plt::rc::params();
    let font = FontProperties::from_rc(&rc).with_size(FontSize::Points(points), rc.font_size);
    TextProps::default().color(color).font(font)
}

/// `label_props={'fontsize': [8, 12, 16], 'color': ['red', 'green',
/// 'blue']}`.
fn styled_labels() -> Vec<TextProps> {
    STYLED_COLORS
        .iter()
        .zip([8.0, 12.0, 16.0])
        .map(|(color, points)| label(color, points))
        .collect()
}

/// Markers edged red, green and blue, filled in their pale shades.
fn styled_markers() -> ButtonMarkerProps {
    ButtonMarkerProps {
        edgecolors: Some(colors(&STYLED_COLORS)),
        facecolors: Some(colors(&["mistyrose", "palegreen", "lightblue"])),
        ..ButtonMarkerProps::default()
    }
}

/// `test_check_radio_buttons_image`: default and styled radio and check
/// buttons beside a drawn axes.
fn check_radio_buttons_image(fig: &Figure) -> CaseResult {
    // `matplotlib.testing.widgets.get_ax`.
    let ax = fig.subplot();
    ax.plot([0.0, 200.0], [0.0, 200.0])?;
    ax.set_aspect(plt::rc::Aspect::Ratio(1.0))?;
    fig.layout();

    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        left: Some(0.3),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    let actives = [false, true, true];
    RadioButtons::new(&fig.add_axes([0.05, 0.7, 0.2, 0.15]), &RADIO_LABELS, 0);
    CheckButtons::new(
        &fig.add_axes([0.05, 0.5, 0.2, 0.15]),
        &CHECK_LABELS,
        &actives,
    );
    let radio = RadioButtonsOptions {
        label_props: styled_labels(),
        radio_props: styled_markers(),
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.05, 0.3, 0.2, 0.15]),
        &RADIO_LABELS,
        0,
        &radio,
    )?;
    let check = CheckButtonsOptions {
        label_props: styled_labels(),
        frame_props: styled_markers(),
        check_props: ButtonMarkerProps {
            facecolors: Some(colors(&STYLED_COLORS)),
            ..ButtonMarkerProps::default()
        },
        ..CheckButtonsOptions::default()
    };
    CheckButtons::with_options(
        &fig.add_axes([0.05, 0.1, 0.2, 0.15]),
        &CHECK_LABELS,
        &actives,
        &check,
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The `label_props` of `test_radio_buttons_props` and
/// `test_check_button_props`: red labels of 24 points.
fn big_red_labels() -> Vec<TextProps> {
    vec![label("red", 24.0)]
}

/// The markers of `test_radio_buttons_props` and the frames of
/// `test_check_button_props`, sized for the 24-point labels.
fn green_blue_markers() -> ButtonMarkerProps {
    ButtonMarkerProps {
        sizes: Some(vec![(24.0_f64 / 2.0).powi(2)]),
        facecolors: Some(colors(&["green"])),
        edgecolors: Some(colors(&["blue"])),
        linewidths: Some(vec![2.0]),
        ..ButtonMarkerProps::default()
    }
}

/// `test_radio_buttons_props`: labels and buttons styled through the
/// setters.
fn radio_buttons_props(fig: &Figure) -> CaseResult {
    let radio = RadioButtons::new(&fig.subplot(), &["tea", "coffee"], 0);
    radio.set_label_props(&big_red_labels());
    radio.set_radio_props(&green_blue_markers())?;
    Ok(())
}

/// `test_radio_grid_buttons`: radio buttons in a row and a column, check
/// buttons in a grid.
fn radio_grid_buttons(fig: &Figure) -> CaseResult {
    let row = RadioButtonsOptions {
        layout: ButtonLayout::Horizontal,
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.1, 0.05, 0.65, 0.05]),
        &["tea", "coffee", "chocolate milk", "water", "soda", "coke"],
        4,
        &row,
    )?;
    let grid = CheckButtonsOptions {
        layout: ButtonLayout::Grid { rows: 3, cols: 2 },
        ..CheckButtonsOptions::default()
    };
    CheckButtons::with_options(
        &fig.add_axes([0.1, 0.15, 0.25, 0.05 * 3.0]),
        &["Chicken", "Salad", "Rice", "Sushi", "Pizza", "Fries"],
        &[true, true, false, false, false, true],
        &grid,
    )?;
    let column = RadioButtonsOptions {
        layout: ButtonLayout::Vertical,
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.1, 0.35, 0.2, 0.05 * 4.0]),
        &["Trinity Cream", "Cake", "Ice Cream", "Muhallebi"],
        3,
        &column,
    )?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_radio_buttons_activecolor_change`: the active colour changed
/// after creation.
fn radio_buttons_activecolor_change(fig: &Figure) -> CaseResult {
    let options = RadioButtonsOptions {
        activecolor: Some("red".into()),
        ..RadioButtonsOptions::default()
    };
    let radio = RadioButtons::with_options(&fig.subplot(), &["tea", "coffee"], 0, &options)?;
    radio.set_activecolor("green")?;
    Ok(())
}

/// `test_check_button_props`: labels, frames and crosses styled through
/// the setters.
fn check_button_props(fig: &Figure) -> CaseResult {
    let checks = CheckButtons::new(&fig.subplot(), &["tea", "coffee"], &[true, true]);
    checks.set_label_props(&big_red_labels());
    checks.set_frame_props(&green_blue_markers());
    checks.set_check_props(&ButtonMarkerProps {
        sizes: Some(vec![(24.0_f64 / 2.0).powi(2)]),
        edgecolors: Some(colors(&["red"])),
        linewidths: Some(vec![2.0]),
        ..ButtonMarkerProps::default()
    })?;
    Ok(())
}

/// `test_rect_visibility`: a rectangle selector asked to be invisible.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn rect_visibility(fig: &Figure) -> CaseResult {
    let options = RectangleSelectorOptions {
        props: PatchProps::default().visible(false),
        ..RectangleSelectorOptions::default()
    };
    RectangleSelector::new(&fig.subplot(), &options).set_extents((0.2, 0.8, 0.3, 0.7));
    Ok(())
}

/// `test_polygon_selector_verts_setter`'s test figure: a polygon selector
/// given its vertices, with or without its bounding box.
fn polygon_selector_verts_setter(fig: &Figure, draw_bounding_box: bool) -> CaseResult {
    let options = PolygonSelectorOptions {
        draw_bounding_box,
        ..PolygonSelectorOptions::default()
    };
    PolygonSelector::new(&fig.subplot(), &options).set_verts(&[
        (0.1, 0.4),
        (0.5, 0.9),
        (0.3, 0.2),
    ])?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "mpl_rendering/check_radio_buttons_image",
        check_radio_buttons_image,
    ),
    classic("mpl_rendering/radio_buttons", |fig| {
        RadioButtons::new(&fig.subplot(), &["tea", "coffee"], 0);
        Ok(())
    }),
    classic("mpl_rendering/radio_buttons_props", radio_buttons_props),
    FigureCase::new("mpl_rendering/radio_grid_buttons", radio_grid_buttons),
    classic(
        "mpl_rendering/radio_buttons_activecolor_change",
        radio_buttons_activecolor_change,
    ),
    classic("mpl_rendering/check_buttons", |fig| {
        CheckButtons::new(&fig.subplot(), &["tea", "coffee"], &[true, true]);
        Ok(())
    }),
    classic("mpl_rendering/check_button_props", check_button_props),
    classic("mpl_rendering/rect_visibility", rect_visibility),
    classic("mpl_rendering/polygon_selector_verts_setter_false", |fig| {
        polygon_selector_verts_setter(fig, false)
    }),
    classic("mpl_rendering/polygon_selector_verts_setter_true", |fig| {
        polygon_selector_verts_setter(fig, true)
    }),
];
