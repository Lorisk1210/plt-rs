//! Buttons: `buttons`, `check_buttons`, `radio_buttons` and
//! `radio_buttons_grid`.

use plt::figure::{MosaicOptions, Share};
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::text::TextProps;
use plt::widgets::{
    Button, ButtonLayout, ButtonMarkerProps, ButtonOptions, CheckButtons, CheckButtonsOptions,
    RadioButtons, RadioButtonsOptions,
};

use super::{sine, t};
use crate::figures::cases::common::{colors, mosaic, sized};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_axisartist/buttons", buttons),
    FigureCase::new("gallery_axisartist/check_buttons", check_buttons),
    FigureCase::new("gallery_axisartist/radio_buttons", radio_buttons),
    FigureCase::new("gallery_axisartist/radio_buttons_grid", radio_buttons_grid)
        .sized(8.0, 4.0, 100.0),
];

fn buttons(fig: &Figure) -> CaseResult {
    let freqs = arange(2.0, 20.0, 3.0);
    let ax = fig.subplot();
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    let t = arange(0.0, 1.0, 0.001);
    ax.plot_with(
        t.clone(),
        sine(&t, freqs[0]),
        LineProps::default().linewidth(2.0),
    )?;
    let axprev = fig.add_axes([0.7, 0.05, 0.1, 0.075]);
    let axnext = fig.add_axes([0.81, 0.05, 0.1, 0.075]);
    let _next = Button::new(&axnext, "Next", ButtonOptions::default());
    let _prev = Button::new(&axprev, "Previous", ButtonOptions::default());
    Ok(())
}

fn check_buttons(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = t();
    let mut lines = Vec::new();
    for (freq, color, label) in [
        (1.0, "black", "1 Hz"),
        (2.0, "red", "2 Hz"),
        (3.0, "blue", "3 Hz"),
    ] {
        let props = LineProps::default().linewidth(2.0).color(color);
        let line = ax.plot_with(t.clone(), sine(&t, freq), props)?;
        line.set_label(label);
        lines.push(line);
    }
    lines[0].set_visible(false);
    let line_colors: Vec<Color> = lines.iter().map(Line2D::color).collect();
    let labels: Vec<String> = lines.iter().map(Line2D::label).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let actives: Vec<bool> = lines.iter().map(Line2D::visible).collect();
    let rax = ax.inset_axes([0.0, 0.0, 0.12, 0.2]);
    let options = CheckButtonsOptions {
        label_props: line_colors
            .iter()
            .map(|color| TextProps::default().color(color.clone()))
            .collect(),
        frame_props: ButtonMarkerProps {
            edgecolors: Some(line_colors.clone()),
            ..ButtonMarkerProps::default()
        },
        check_props: ButtonMarkerProps {
            facecolors: Some(line_colors),
            ..ButtonMarkerProps::default()
        },
        ..CheckButtonsOptions::default()
    };
    CheckButtons::with_options(&rax, &labels, &actives, options)?;
    Ok(())
}

/// The light grey panel with a title that holds radio buttons.
fn button_panel(ax: &Axes, title: &str) {
    ax.set_facecolor("0.95");
    for spine in ax.spines().iter() {
        spine.set_color("0.8");
    }
    ax.set_title(title);
}

fn radio_buttons(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let options = MosaicOptions {
        width_ratios: Some(vec![5.0, 1.0]),
        ..MosaicOptions::default()
    };
    let layout = mosaic(&[
        &["main", "freq"],
        &["main", "color"],
        &["main", "linestyle"],
    ])?;
    let axd = fig.subplot_mosaic(&layout, options)?;
    let get = |label| axd.get(label).ok_or("a mosaic cell");
    let main = get("main")?;
    let t = t();
    let props = LineProps::default().linewidth(2.0).color("red");
    main.plot_with(t.clone(), sine(&t, 1.0), props)?;
    main.set_xlabel("Time (s)")
        .set_ylabel("Amplitude")
        .set_title("Sine Wave");

    let freq = get("freq")?;
    button_panel(freq, "Frequency");
    let frequencies = RadioButtonsOptions {
        label_props: [12.0, 14.0, 16.0]
            .map(|size| TextProps::default().font(sized(size)))
            .to_vec(),
        radio_props: ButtonMarkerProps {
            sizes: Some(vec![16.0, 32.0, 64.0]),
            ..ButtonMarkerProps::default()
        },
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(freq, &["1 Hz", "2 Hz", "4 Hz"], 0, frequencies)?;

    let color = get("color")?;
    button_panel(color, "Color");
    let names = ["red", "blue", "green"];
    let colored = RadioButtonsOptions {
        label_props: names.map(|name| TextProps::default().color(name)).to_vec(),
        radio_props: ButtonMarkerProps {
            facecolors: Some(colors(&names)),
            edgecolors: Some(colors(&["darkred", "darkblue", "darkgreen"])),
            ..ButtonMarkerProps::default()
        },
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(color, &names, 0, colored)?;

    let linestyle = get("linestyle")?;
    button_panel(linestyle, "Linestyle");
    RadioButtons::new(linestyle, &["solid", "dashed", "dashdot", "dotted"], 0);
    Ok(())
}

fn radio_buttons_grid(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        1,
        2,
        Some(vec![4.0, 1.4]),
        None,
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&grid, Share::None, Share::None)?;
    let (ax, ax_buttons) = (&axs[(0, 0)], &axs[(0, 1)]);
    let t = t();
    let props = LineProps::default().linewidth(2.0).color("red");
    ax.plot_with(t.clone(), sine(&t, 1.0), props)?;
    ax.set_xlabel("Time (s)")
        .set_ylabel("Amplitude")
        .set_title("Sine Wave - Click a color!");
    button_panel(ax_buttons, "Line Color");
    let options = RadioButtonsOptions {
        layout: ButtonLayout::Grid { rows: 3, cols: 2 },
        ..RadioButtonsOptions::default()
    };
    RadioButtons::with_options(
        ax_buttons,
        &["red", "yellow", "green", "purple", "brown", "gray"],
        0,
        options,
    )?;
    Ok(())
}
