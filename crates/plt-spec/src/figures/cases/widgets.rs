//! `figures/cases_widgets.py`: widgets drawn in their default and styled
//! states.

use crate::figures::cases::common::{colors, sized};
use plt::colors::Color;
use plt::fonts::FontWeight;
use plt::patches::PatchProps;
use plt::prelude::*;
use plt::text::{HAlign, TextProps};
use plt::widgets::{
    Button, ButtonLayout, ButtonMarkerProps, ButtonOptions, CheckButtons, CheckButtonsOptions,
    HandleStyle, Orientation, RadioButtons, RadioButtonsOptions, RangeSlider, Slider,
    SliderOptions, TextBox, TextBoxOptions, ValStep,
};

use crate::figures::{CaseResult, FigureCase};

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("widgets/check_buttons_styled_props", check_styled).sized(4.0, 3.0, 100.0),
    FigureCase::new("widgets/radio_buttons_styled_props", radio_styled).sized(4.0, 3.0, 100.0),
    FigureCase::new("widgets/radio_buttons_activecolor_layouts", radio_layouts)
        .sized(6.0, 3.0, 100.0),
    FigureCase::new("widgets/check_buttons_layouts", check_layouts).sized(6.0, 3.0, 100.0),
    FigureCase::new("widgets/slider_handle_style_poly_props", slider_styles).sized(5.0, 3.0, 100.0),
    FigureCase::new("widgets/slider_vertical_closed_ends", slider_vertical).sized(4.0, 4.0, 100.0),
    FigureCase::new("widgets/range_slider_styles", range_sliders).sized(5.0, 4.0, 100.0),
    FigureCase::new("widgets/button_styles", buttons).sized(5.0, 2.0, 100.0),
    FigureCase::new("widgets/textbox_alignments_colors", textboxes).sized(5.0, 3.0, 100.0),
];

/// `COLORS`.
const COLORS: [&str; 4] = ["tab:red", "tab:green", "tab:blue", "k"];

fn check_styled(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
    let label_props = COLORS
        .iter()
        .zip([8.0, 10.0, 12.0, 14.0])
        .map(|(&color, size)| TextProps::default().color(color).font(sized(size)))
        .collect();
    let options = CheckButtonsOptions {
        label_props,
        frame_props: ButtonMarkerProps {
            edgecolors: Some(colors(&COLORS)),
            facecolors: Some(colors(&["0.95"])),
            sizes: Some(vec![40.0, 60.0, 80.0, 100.0]),
            linewidths: Some(vec![2.0]),
            ..ButtonMarkerProps::default()
        },
        check_props: ButtonMarkerProps {
            facecolors: Some(colors(&COLORS)),
            linewidths: Some(vec![2.0]),
            ..ButtonMarkerProps::default()
        },
        ..CheckButtonsOptions::default()
    };
    let labels = ["red", "green", "blue", "black"];
    CheckButtons::with_options(&ax, &labels, &[true, false, true, false], options)?;
    Ok(())
}

fn radio_styled(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
    ax.set_facecolor("ivory");
    let label_props = [
        ("0.4", 9.0, false),
        ("k", 11.0, false),
        ("tab:red", 13.0, true),
    ]
    .into_iter()
    .map(|(color, size, bold)| {
        let font = if bold {
            sized(size).with_weight(FontWeight::BOLD)
        } else {
            sized(size)
        };
        TextProps::default().color(color).font(font)
    })
    .collect();
    let options = RadioButtonsOptions {
        label_props,
        radio_props: ButtonMarkerProps {
            facecolors: Some(colors(&["tab:blue", "tab:orange", "tab:red"])),
            edgecolors: Some(colors(&["0.3"])),
            sizes: Some(vec![30.0, 60.0, 120.0]),
            ..ButtonMarkerProps::default()
        },
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(&ax, &["slow", "medium", "fast"], 2, options)?;
    Ok(())
}

fn radio_layouts(fig: &Figure) -> CaseResult {
    let across = RadioButtonsOptions {
        layout: ButtonLayout::Horizontal,
        activecolor: Some(Color::from("tab:green")),
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.05, 0.55, 0.9, 0.35])?,
        &["one", "two", "three", "four"],
        1,
        across,
    )?;
    let grid = RadioButtonsOptions {
        layout: ButtonLayout::Grid { rows: 2, cols: 3 },
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.05, 0.1, 0.4, 0.35])?,
        &["a", "bb", "ccc", "dddd", "e"],
        3,
        grid,
    )?;
    let column = RadioButtonsOptions {
        layout: ButtonLayout::Vertical,
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        &fig.add_axes([0.55, 0.1, 0.4, 0.35])?,
        &["top", "middle", "bottom"],
        0,
        column,
    )?;
    Ok(())
}

fn check_layouts(fig: &Figure) -> CaseResult {
    let across = CheckButtonsOptions {
        layout: ButtonLayout::Horizontal,
        ..CheckButtonsOptions::default()
    };
    CheckButtons::with_options(
        &fig.add_axes([0.05, 0.55, 0.9, 0.35])?,
        &["alpha", "beta", "gamma"],
        &[true, false, true],
        across,
    )?;
    let grid = CheckButtonsOptions {
        layout: ButtonLayout::Grid { rows: 2, cols: 3 },
        label_props: vec![TextProps::default().font(sized(9.0))],
        ..CheckButtonsOptions::default()
    };
    let actives: Vec<bool> = (0..6).map(|i| i % 2 == 0).collect();
    CheckButtons::with_options(
        &fig.add_axes([0.05, 0.1, 0.9, 0.35])?,
        &["A", "B", "C", "D", "E", "F"],
        &actives,
        grid,
    )?;
    Ok(())
}

fn slider_styles(fig: &Figure) -> CaseResult {
    let hz = SliderOptions {
        valinit: 12.5,
        valfmt: Some("%1.1f Hz".into()),
        handle_style: HandleStyle {
            facecolor: Color::from("tab:orange"),
            edgecolor: Color::from("k"),
            size: 14.0,
        },
        track_color: Color::from("0.92"),
        initcolor: Color::none(),
        poly: PatchProps::default().facecolor("tab:green").alpha(0.6),
        ..SliderOptions::default()
    };
    Slider::new(&fig.add_axes([0.25, 0.75, 0.55, 0.1])?, "Hz", 0.0, 50.0, hz)?;
    let gain = SliderOptions {
        valinit: -3.0,
        valstep: Some(ValStep::Values(vec![
            -10.0, -6.0, -3.0, 0.0, 3.0, 6.0, 10.0,
        ])),
        initcolor: Color::from("tab:blue"),
        poly: PatchProps::default()
            .facecolor("tab:purple")
            .edgecolor("k")
            .linewidth(1.0)
            .hatch("//"),
        ..SliderOptions::default()
    };
    Slider::new(
        &fig.add_axes([0.25, 0.5, 0.55, 0.1])?,
        "Gain",
        -10.0,
        10.0,
        gain,
    )?;
    let big = SliderOptions {
        valinit: 7.5e5,
        ..SliderOptions::default()
    };
    Slider::new(
        &fig.add_axes([0.25, 0.25, 0.55, 0.1])?,
        "Big",
        0.0,
        2e6,
        big,
    )?;
    Ok(())
}

fn slider_vertical(fig: &Figure) -> CaseResult {
    let lo = SliderOptions {
        valinit: 0.0,
        closedmin: false,
        orientation: Orientation::Vertical,
        ..SliderOptions::default()
    };
    Slider::new(&fig.add_axes([0.15, 0.15, 0.08, 0.7])?, "lo", 0.0, 1.0, lo)?;
    let mid = SliderOptions {
        valinit: 37.0,
        valstep: Some(ValStep::Step(10.0)),
        orientation: Orientation::Vertical,
        track_color: Color::from("lightblue"),
        handle_style: HandleStyle {
            size: 6.0,
            ..HandleStyle::default()
        },
        ..SliderOptions::default()
    };
    Slider::new(
        &fig.add_axes([0.45, 0.15, 0.08, 0.7])?,
        "mid",
        0.0,
        100.0,
        mid,
    )?;
    let hi = SliderOptions {
        valinit: 2.0,
        orientation: Orientation::Vertical,
        initcolor: Color::none(),
        ..SliderOptions::default()
    };
    Slider::new(&fig.add_axes([0.75, 0.15, 0.08, 0.7])?, "hi", -1.0, 1.0, hi)?;
    Ok(())
}

fn range_sliders(fig: &Figure) -> CaseResult {
    let defaults = SliderOptions::default();
    RangeSlider::new(
        &fig.add_axes([0.25, 0.8, 0.6, 0.08])?,
        "default",
        0.0,
        10.0,
        None,
        defaults.clone(),
    )?;
    let styled = SliderOptions {
        valfmt: Some("%+.1f".into()),
        handle_style: HandleStyle {
            facecolor: Color::from("tab:red"),
            size: 8.0,
            ..HandleStyle::default()
        },
        poly: PatchProps::default().facecolor("tab:red").alpha(0.4),
        track_color: Color::from("mistyrose"),
        ..SliderOptions::default()
    };
    RangeSlider::new(
        &fig.add_axes([0.25, 0.6, 0.6, 0.08])?,
        "styled",
        -5.0,
        5.0,
        Some((-4.0, 1.0)),
        styled,
    )?;
    RangeSlider::new(
        &fig.add_axes([0.25, 0.4, 0.6, 0.08])?,
        "swapped",
        0.0,
        1.0,
        Some((0.9, 0.2)),
        defaults,
    )?;
    let vertical = SliderOptions {
        orientation: Orientation::Vertical,
        valstep: Some(ValStep::Step(0.05)),
        poly: PatchProps::default().edgecolor("k"),
        ..SliderOptions::default()
    };
    RangeSlider::new(
        &fig.add_axes([0.08, 0.1, 0.06, 0.4])?,
        "vert",
        0.0,
        1.0,
        Some((0.1, 0.4)),
        vertical,
    )?;
    Ok(())
}

fn buttons(fig: &Figure) -> CaseResult {
    Button::new(
        &fig.add_axes([0.05, 0.3, 0.25, 0.4])?,
        "Default",
        ButtonOptions::default(),
    );
    let styled = Button::new(
        &fig.add_axes([0.375, 0.3, 0.25, 0.4])?,
        "Styled",
        ButtonOptions {
            color: Color::from("tab:blue"),
            hovercolor: Color::from("tab:cyan"),
        },
    );
    styled
        .label()
        .set_color("w")
        .set_weight(FontWeight::BOLD)
        .set_fontsize(14.0);
    let framed = Button::new(
        &fig.add_axes([0.7, 0.3, 0.25, 0.4])?,
        "Framed\ntwo lines",
        ButtonOptions {
            color: Color::from("lightyellow"),
            ..ButtonOptions::default()
        },
    );
    for spine in framed.ax().spines().iter() {
        spine.set_color("tab:orange").set_linewidth(2.0);
    }
    Ok(())
}

fn textboxes(fig: &Figure) -> CaseResult {
    TextBox::new(
        &fig.add_axes([0.25, 0.75, 0.6, 0.12])?,
        "left ",
        "aligned left",
        TextBoxOptions::default(),
    );
    TextBox::new(
        &fig.add_axes([0.25, 0.5, 0.6, 0.12])?,
        "center ",
        "centred",
        TextBoxOptions {
            textalignment: HAlign::Center,
            color: Color::from("lightcyan"),
            ..TextBoxOptions::default()
        },
    );
    TextBox::new(
        &fig.add_axes([0.25, 0.25, 0.6, 0.12])?,
        "right",
        "right $x^2$",
        TextBoxOptions {
            textalignment: HAlign::Right,
            label_pad: 0.05,
            hovercolor: Color::from("w"),
            ..TextBoxOptions::default()
        },
    );
    TextBox::new(
        &fig.add_axes([0.25, 0.05, 0.6, 0.12])?,
        "empty",
        "",
        TextBoxOptions::default(),
    );
    Ok(())
}
