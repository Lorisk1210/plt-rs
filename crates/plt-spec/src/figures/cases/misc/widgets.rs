//! Widget cases of `figures/cases_axes_misc.py`.

use plt::colors::Color;
use plt::widgets::{
    Button, ButtonOptions, CheckButtons, Orientation, RadioButtons, RangeSlider, Slider,
    SliderOptions, TextBox, TextBoxOptions, ValStep,
};

use crate::figures::FigureCase;

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/widgets_sliders_button", |fig| {
        let freq = SliderOptions {
            valinit: 5.0,
            ..SliderOptions::default()
        };
        let _s1 = Slider::new(
            &fig.add_axes([0.2, 0.8, 0.6, 0.08]),
            "Freq",
            0.1,
            30.0,
            freq,
        );
        let amp = SliderOptions {
            valinit: 7.0,
            valstep: Some(ValStep::Step(1.0)),
            orientation: Orientation::Vertical,
            ..SliderOptions::default()
        };
        let _s2 = Slider::new(&fig.add_axes([0.05, 0.1, 0.05, 0.6]), "Amp", 0.0, 10.0, amp);
        let _s3 = RangeSlider::new(
            &fig.add_axes([0.2, 0.6, 0.6, 0.08]),
            "Range",
            0.0,
            1.0,
            Some((0.2, 0.7)),
            SliderOptions::default(),
        );
        let reset = ButtonOptions {
            color: Color::from("0.9"),
            ..ButtonOptions::default()
        };
        let _b = Button::new(&fig.add_axes([0.6, 0.15, 0.25, 0.15]), "Reset", reset);
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/widgets_check_radio_textbox", |fig| {
        let _c = CheckButtons::new(
            &fig.add_axes([0.05, 0.3, 0.25, 0.5]),
            &["red", "green", "blue"],
            &[true, false, true],
        );
        let _r = RadioButtons::new(
            &fig.add_axes([0.38, 0.3, 0.25, 0.5]),
            &["one", "two", "three"],
            1,
        );
        let _t = TextBox::new(
            &fig.add_axes([0.78, 0.45, 0.18, 0.1]),
            "Name ",
            "plt-rs",
            TextBoxOptions::default(),
        );
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
