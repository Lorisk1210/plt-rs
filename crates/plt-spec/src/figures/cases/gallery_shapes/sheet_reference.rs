//! `style_sheets_reference`: the same six demonstration axes (scatter,
//! image, bars, coloured lines, histograms and circles) under every
//! bundled style sheet, one case per sheet.

use plt::axes::Adjustable;
use plt::fonts::{FontWeight, RelativeSize};
use plt::layout::LayoutEngine;
use plt::patches::{Circle, Rectangle};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, ArrowProps, HAlign, VAlign};

use super::sheets::{beta_histograms, image_and_patch, sigmoids};
use crate::figures::cases::common::{arrow, styled_box};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The suptitle colour: light on dark figures, dark blue otherwise.
fn title_color() -> Result<Color, Box<dyn std::error::Error>> {
    let face = plt::rc::params().figure_facecolor.resolve()?;
    Ok(if face.r.max(face.g).max(face.b) < 0.5 {
        Color::from((0.8, 0.8, 1.0))
    } else {
        Color::from((19.0 / 256.0, 6.0 / 256.0, 84.0 / 256.0))
    })
}

/// `plot_scatter`: two clouds of normal samples, circles and squares.
fn scatter_clouds(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    for (mu, sigma, marker) in [(-0.5, 0.75, "o"), (0.75, 1.0, "s")] {
        let xy = rng.normal(2 * 100, mu, sigma);
        let props = LineProps::default()
            .linestyle(LineStyle::None)
            .marker(marker.parse::<Marker>()?);
        ax.plot_with(xy[..100].to_vec(), xy[100..].to_vec(), props)?;
    }
    ax.set_xlabel("X-label");
    ax.set_title("Axes title");
    Ok(())
}

/// `plot_bar_graphs`: two series of five random bars side by side.
fn bar_graphs(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    let x = arange(0.0, 5.0, 1.0);
    let heights: Vec<f64> = rng
        .integers(2 * 5, 5, 25)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let width = 0.25;
    let narrow = BarOptions {
        thickness: vec![width],
        ..BarOptions::default()
    };
    ax.bar(x.clone(), &heights[..5], narrow.clone())?;
    let shifted: Vec<f64> = x.iter().map(|x| x + width).collect();
    let green = BarOptions {
        color: vec!["C2".into()],
        ..narrow
    };
    ax.bar(shifted.clone(), &heights[5..], green)?;
    ax.set_xticks(shifted)
        .set_xticklabels(["a", "b", "c", "d", "e"])?;
    Ok(())
}

/// `plot_histograms`: the beta histograms with an annotation.
fn annotated_histograms(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    beta_histograms(ax, rng, 30)?;
    let options = AnnotateOptions {
        xytext: Some((0.9, 0.9).into()),
        textcoords: Some("axes fraction".parse()?),
        arrowprops: Some(ArrowProps::Fancy(arrow(
            "->",
            "angle,angleA=-95,angleB=35,rad=10",
        )?)),
        ..AnnotateOptions::default()
    };
    ax.annotate("Annotation", (0.25, 4.25), options)?
        .set_valign(VAlign::Top)
        .set_halign(HAlign::Right)
        .set_bbox(styled_box("round", PatchProps::default().alpha(0.2))?);
    Ok(())
}

/// `plot_colored_circles`: fifteen circles in the colours of the cycle on
/// a grid.
fn colored_circles(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    let colors = plt::rc::params().axes_prop_cycle.colors();
    for color in colors.iter().cycle().take(15) {
        let xy = rng.normal(2, 0.0, 3.0);
        ax.add_patch_with(
            Circle::new((xy[0], xy[1]), 1.0),
            PatchProps::default().color(*color),
        );
    }
    ax.grid(true);
    ax.set_title("ax.grid(True)")
        .set_family(["monospace"])
        .set_fontsize(RelativeSize::Small);
    ax.set_xlim(-4.0, 8.0).set_ylim(-5.0, 6.0);
    ax.set_aspect(Aspect::Equal)?
        .set_adjustable(Adjustable::Box)?;
    Ok(())
}

/// `plot_figure`: the demonstration figure under the active sheet, titled
/// `style_label`.
fn style_reference(fig: &Figure, style_label: &str) -> CaseResult {
    let mut rng = SpecRng::new(96_917_002);
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(1, 6)?;
    fig.suptitle(style_label)
        .set_x(0.01)
        .set_halign(HAlign::Left)
        .set_color(title_color()?)
        .set_fontsize(14.0)
        .set_family(["DejaVu Sans"])
        .set_weight(FontWeight::NORMAL);
    let ax = |col: usize| &axs[(0, col)];
    scatter_clouds(ax(0), &mut rng)?;
    image_and_patch(ax(1), &mut rng)?;
    ax(1)
        .set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    bar_graphs(ax(2), &mut rng)?;
    sigmoids(ax(3), false)?;
    annotated_histograms(ax(4), &mut rng)?;
    colored_circles(ax(5), &mut rng)?;
    let divider = PatchProps::default().clip_on(false).color("gray");
    ax(4).add_artist_with(Rectangle::new((1.0 + 0.025, -2.0), 0.05, 16.0), divider);
    Ok(())
}

/// Parameters pinned on top of each sheet so that goldens do not depend
/// on the host (`PINNED`).
const PINNED: &[(&str, &str)] = &[("font.family", "DejaVu Sans"), ("text.hinting", "none")];

/// One `style_sheets_reference_<sheet>` case per bundled sheet, drawn
/// under `{**library[sheet], **PINNED}`.
macro_rules! sheet_cases {
    ($($name:literal),* $(,)?) => {
        &[
            FigureCase::new("gallery_shapes/style_sheets_reference_default", |fig| {
                style_reference(fig, "default")
            })
            .sized(14.8, 2.8, 100.0),
            $(
                FigureCase::new(concat!("gallery_shapes/style_sheets_reference_", $name), |fig| {
                    style_reference(fig, $name)
                })
                .sized(14.8, 2.8, 100.0)
                .with_style($name)
                .with_rc(PINNED),
            )*
        ]
    };
}

/// `style_list`: `default`, then `classic`, then every other public sheet
/// in alphabetical order.
pub(super) const CASES: &[FigureCase] = sheet_cases![
    "classic",
    "Solarize_Light2",
    "bmh",
    "dark_background",
    "fast",
    "fivethirtyeight",
    "ggplot",
    "grayscale",
    "petroff10",
    "petroff6",
    "petroff8",
    "seaborn-v0_8",
    "seaborn-v0_8-bright",
    "seaborn-v0_8-colorblind",
    "seaborn-v0_8-dark",
    "seaborn-v0_8-dark-palette",
    "seaborn-v0_8-darkgrid",
    "seaborn-v0_8-deep",
    "seaborn-v0_8-muted",
    "seaborn-v0_8-notebook",
    "seaborn-v0_8-paper",
    "seaborn-v0_8-pastel",
    "seaborn-v0_8-poster",
    "seaborn-v0_8-talk",
    "seaborn-v0_8-ticks",
    "seaborn-v0_8-white",
    "seaborn-v0_8-whitegrid",
    "tableau-colorblind10",
];
