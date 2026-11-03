//! Fancy box and arrow cases of `figures/cases_patches.py`.

use pltrs::coords::Coords;
use pltrs::patches::{
    Arrow, ArrowShape, ArrowStyle, BoxStyle, ConnectionEnd, ConnectionPatch, ConnectionStyle,
    FancyArrow, FancyArrowPatch, FancyBboxPatch,
};
use pltrs::prelude::*;
use pltrs::text::HAlign;
use pltrs::transforms::Point;
use pltrs_core::{Path, PathCode};

use super::{canvas, square_canvas};
use crate::figures::FigureCase;

/// `BOXSTYLES`: every box style.
const BOXSTYLES: [&str; 10] = [
    "square",
    "round",
    "round4",
    "sawtooth",
    "roundtooth",
    "circle",
    "ellipse",
    "darrow",
    "larrow",
    "rarrow",
];

/// A `FancyArrowPatch((x0, y0), (x1, y1), arrowstyle=style, mutation_scale=scale)`.
fn arrow_between(
    a: (f64, f64),
    b: (f64, f64),
    style: &str,
    scale: f64,
) -> Result<FancyArrowPatch, Box<dyn std::error::Error>> {
    Ok(FancyArrowPatch::between(a, b)
        .with_arrowstyle(style.parse::<ArrowStyle>()?)
        .with_mutation_scale(scale))
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("patches/fancybbox_boxstyles", |fig| {
        let ax = canvas(fig, (0.0, 20.0), (0.0, 12.0), true)?;
        for (i, style) in BOXSTYLES.into_iter().enumerate() {
            let (x, y) = (1.2 + 4.0 * (i % 5) as f64, 7.0 - 5.5 * (i / 5) as f64);
            let boxstyle: BoxStyle = format!("{style},pad=0.2").parse()?;
            ax.add_patch(FancyBboxPatch::new((x, y), 1.6, 0.8, boxstyle))
                .set_facecolor("lightsteelblue")
                .set_edgecolor("navy");
            ax.text(x + 0.8, y + 2.8, style)
                .set_halign(HAlign::Center)
                .set_fontsize(8.0);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("patches/fancybbox_mutation", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false)?;
        let round: BoxStyle = "round,pad=0.3".parse()?;
        ax.add_patch(FancyBboxPatch::new((1.0, 3.0), 2.0, 4.0, round.clone()));
        ax.add_patch(
            FancyBboxPatch::new((4.0, 3.0), 2.0, 4.0, round.clone()).with_mutation_scale(2.0),
        )
        .set_facecolor("C1");
        ax.add_patch(FancyBboxPatch::new((7.0, 3.0), 2.0, 4.0, round).with_mutation_aspect(3.0))
            .set_facecolor("C2");
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("patches/fancyarrow", |fig| {
        let ax = square_canvas(fig)?;
        ax.add_patch(FancyArrow {
            width: 0.3,
            ..FancyArrow::new(1.0, 8.0, 6.0, 0.0)
        });
        ax.add_patch(FancyArrow {
            width: 0.3,
            head_width: Some(1.2),
            head_length: Some(1.5),
            length_includes_head: true,
            ..FancyArrow::new(1.0, 6.0, 6.0, 0.0)
        })
        .set_facecolor("C1");
        ax.add_patch(FancyArrow {
            width: 0.3,
            shape: ArrowShape::Left,
            overhang: 0.4,
            ..FancyArrow::new(1.0, 4.0, 6.0, 0.0)
        })
        .set_facecolor("C2");
        ax.add_patch(FancyArrow {
            width: 0.3,
            shape: ArrowShape::Right,
            head_starts_at_zero: true,
            ..FancyArrow::new(1.0, 2.0, 6.0, 0.0)
        })
        .set_facecolor("C3");
        ax.add_patch(FancyArrow {
            width: 0.2,
            ..FancyArrow::new(9.0, 1.0, 0.0, 7.0)
        })
        .set_facecolor("C4")
        .set_edgecolor("k");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/arrow", |fig| {
        let ax = square_canvas(fig)?;
        ax.add_patch(Arrow::new(1.0, 1.0, 5.0, 5.0));
        ax.add_patch(Arrow::new(2.0, 8.0, 6.0, 0.0).with_width(2.0))
            .set_facecolor("tab:red");
        ax.add_patch(Arrow::new(9.0, 1.0, -3.0, 3.0).with_width(0.5))
            .set_facecolor("tab:green")
            .set_edgecolor("k");
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/fancyarrowpatch_styles", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false)?;
        ax.set_axis_off();
        let styles = [
            "-", "->", "-[", "|-|", "-|>", "<-", "<->", "<|-", "<|-|>", "fancy", "simple", "wedge",
            "]-[", "]->",
        ];
        for (i, style) in styles.into_iter().enumerate() {
            let (x, y) = (0.5 + 5.0 * (i / 7) as f64, 9.3 - 1.4 * (i % 7) as f64);
            ax.add_patch(arrow_between((x, y), (x + 4.0, y), style, 15.0)?)
                .set_facecolor("tab:cyan");
        }
        Ok(())
    })
    .sized(6.0, 5.0, 100.0),
    FigureCase::new("patches/fancyarrowpatch_connections", |fig| {
        let ax = canvas(fig, (0.0, 10.0), (0.0, 10.0), false)?;
        let styles = [
            "arc3,rad=0.4",
            "arc3,rad=-0.4",
            "angle3,angleA=0,angleB=90",
            "angle,angleA=0,angleB=90,rad=10",
            "arc,angleA=0,angleB=0,armA=20,armB=20,rad=5",
            "bar,fraction=-0.3",
        ];
        for (i, style) in styles.into_iter().enumerate() {
            let (x, y) = (0.5 + 3.3 * (i % 3) as f64, 5.5 - 5.0 * (i / 3) as f64);
            ax.plot_fmt([x, x + 2.5], [y, y + 3.5], "k.")?;
            let arrow = arrow_between((x, y), (x + 2.5, y + 3.5), "-|>", 12.0)?
                .with_connectionstyle(style.parse::<ConnectionStyle>()?);
            ax.add_patch(arrow);
        }
        Ok(())
    })
    .sized(6.0, 4.0, 100.0),
    FigureCase::new("patches/fancyarrowpatch_path_shrink", |fig| {
        let ax = square_canvas(fig)?;
        let path = Path::new(
            vec![
                Point::new(1.0, 1.0),
                Point::new(5.0, 10.0),
                Point::new(9.0, 1.0),
            ],
            vec![PathCode::MoveTo, PathCode::Curve3, PathCode::Curve3],
        )?;
        let fancy = FancyArrowPatch::along(path)
            .with_arrowstyle("fancy".parse::<ArrowStyle>()?)
            .with_mutation_scale(25.0);
        ax.add_patch(fancy)
            .set_facecolor("tab:red")
            .set_alpha(Some(0.7));
        ax.add_patch(arrow_between((1.0, 9.0), (9.0, 9.0), "<->", 15.0)?.with_shrink(20.0, 40.0));
        ax.plot_fmt([1.0, 9.0], [9.0, 9.0], "k.")?;
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("patches/connectionpatch_two_axes", |fig| {
        let axs = fig.subplots(1, 2)?;
        let [ax1, ax2] = axs.as_slice() else {
            return Err("two axes expected".into());
        };
        ax1.plot_fmt([0.0, 1.0], [0.0, 1.0], "o-")?;
        ax2.plot_fmt([0.0, 1.0], [1.0, 0.0], "s-")?;
        let mut across = ConnectionPatch::new(
            ConnectionEnd::new((1.0, 1.0), Coords::Data, Some(ax1.id())),
            ConnectionEnd::new((0.0, 1.0), Coords::Data, Some(ax2.id())),
        );
        across.spec.arrowstyle = "->".parse()?;
        ax2.add_artist(across)
            .set_color("tab:red")
            .set_linewidth(2.0);
        let mut curved = ConnectionPatch::new(
            ConnectionEnd::new((0.5, 0.2), Coords::Transform(ax1.trans_axes()), None),
            ConnectionEnd::new((0.5, 0.2), Coords::Transform(ax2.trans_axes()), None),
        );
        curved.spec.connectionstyle = "arc3,rad=-0.3".parse()?;
        fig.add_artist(curved).set_linestyle(LineStyle::Dashed);
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
