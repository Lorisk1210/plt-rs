//! `figures/cases_mpl_tests_widgets.py`: matplotlib's own tests of widgets
//! (`test_widgets.py`) that draw a figure and only check that drawing or
//! saving it works, one case per figure, in the test suite's default
//! style.

use pltrs::artist::ArtistHandle;
use pltrs::figure::{Figure, Share};
use pltrs::fonts::{FontProperties, FontSize};
use pltrs::gridspec::{GridSpec, SubplotOverrides};
use pltrs::text::TextProps;
use pltrs::widgets::{
    ButtonMarkerProps, CheckButtons, CheckButtonsOptions, RadioButtons, RadioButtonsOptions,
};

use super::mpl_tests::classic;
use crate::figures::cases::common::colors;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic(
            "mpl_tests_widgets/save_blitted_widget_as_pdf",
            save_blitted_widget_as_pdf,
        )
        .sized(5.0, 2.0, 100.0),
        classic("mpl_tests_widgets/parent_axes_removal", parent_axes_removal),
    ]
}

/// The buttons' labels.
const FRUITS: [&str; 2] = ["Apples", "Oranges"];

/// `label_props={'color': ['red', 'orange'], 'fontsize': [16, 20]}`.
fn styled_labels() -> Vec<TextProps> {
    let rc = pltrs::rc::params();
    [("red", 16.0), ("orange", 20.0)]
        .into_iter()
        .map(|(color, points)| {
            let font =
                FontProperties::from_rc(&rc).with_size(FontSize::Points(points), rc.font_size);
            TextProps::default().color(color).font(font)
        })
        .collect()
}

/// Markers edged red and orange, filled in their pale shades.
fn styled_markers() -> ButtonMarkerProps {
    ButtonMarkerProps {
        edgecolors: Some(colors(&["red", "orange"])),
        facecolors: Some(colors(&["mistyrose", "peachpuff"])),
        ..ButtonMarkerProps::default()
    }
}

/// `test_save_blitted_widget_as_pdf`: default and styled radio and check
/// buttons on a grid of axes with width ratios 1 and 2.
fn save_blitted_widget_as_pdf(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        2,
        2,
        Some(vec![1.0, 2.0]),
        None,
        SubplotOverrides::default(),
    )?;
    let ax = fig.subplots_on(&grid, Share::None, Share::None)?;
    RadioButtons::new(&ax[(0, 0)], &FRUITS, 0);
    let radio = RadioButtonsOptions {
        label_props: styled_labels(),
        radio_props: styled_markers(),
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(&ax[(0, 1)], &FRUITS, 0, radio)?;
    CheckButtons::new(&ax[(1, 0)], &FRUITS, &[true, true]);
    let check = CheckButtonsOptions {
        label_props: styled_labels(),
        frame_props: styled_markers(),
        check_props: ButtonMarkerProps {
            facecolors: Some(colors(&["darkred", "darkorange"])),
            ..ButtonMarkerProps::default()
        },
        ..CheckButtonsOptions::default()
    };
    CheckButtons::with_options(&ax[(1, 1)], &FRUITS, &[true, true], check)?;
    ax[(0, 0)].set_title("Default");
    ax[(0, 1)].set_title("Stylized");
    Ok(())
}

/// `test_parent_axes_removal`: radio and check buttons whose axes are
/// removed again.
fn parent_axes_removal(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax_radio, ax_checks) = (&axs[(0, 0)], &axs[(0, 1)]);
    let _radio = RadioButtons::new(ax_radio, &["1", "2"], 0);
    let _checks = CheckButtons::new(ax_checks, &["1", "2"], &[true, false]);
    ax_checks.remove();
    ax_radio.remove();
    Ok(())
}
