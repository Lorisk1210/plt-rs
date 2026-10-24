//! The cases from `test_patheffects.py`: strokes, shadows, patch effects
//! and ticked strokes on lines, text, boxes, legends, gridlines, contours
//! and their labels.

use plt::axes::{AxisMode, GridOptions, ImshowOptions};
use plt::contour::{ClabelFormat, ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
use plt::image::{ImageData, Interpolation, ScalarField};
use plt::legend::{LegendHandle, LegendLoc, LegendOptions};
use plt::numeric::{Grid, linspace, meshgrid};
use plt::offsetbox::AnchorLoc;
use plt::patches::PathShape;
use plt::patheffects::{
    GcUpdate, LineShadow, PatchEffect, PatchShadow, PathEffect, Stroke, TickedStroke,
};
use plt::prelude::*;
use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps, HAlign, TextBoxProps, VAlign};
use plt::transforms::Point;
use plt_core::Path;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::cases::common::{
    contour_effects, hatch_shadow_text, stroke, ticked_stroke, with_stroke,
};
use crate::figures::{CaseResult, FigureCase};

/// `test_patheffect1`: a stroked annotation with a stroked arrow over an
/// image, and stroked gridlines.
fn patheffect1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let image = Grid::from_vec(2, 2, vec![1_i64, 2, 2, 3])?;
    ax.imshow(
        ImageData::from(ScalarField::integers(&image)),
        ImshowOptions::default(),
    )?;
    let arrow_effects = vec![PathEffect::stroke(stroke(5.0, "w")), PathEffect::normal()];
    let arrow = FancyArrowProps {
        connectionstyle: "angle3".parse()?,
        patch: PatchProps::default()
            .linewidth(2.0)
            .path_effects(arrow_effects),
        ..FancyArrowProps::new("->".parse()?)
    };
    let options = AnnotateOptions {
        xytext: Some(Point::new(0.0, 0.0).into()),
        arrowprops: Some(ArrowProps::Fancy(arrow)),
        ..AnnotateOptions::default()
    };
    ax.annotate("test", (1.0, 1.0), options)?
        .set_fontsize(20.0)
        .set_halign(HAlign::Center)
        .set_path_effects(with_stroke(3.0, "w"));
    let grid = GridOptions {
        linestyle: Some(LineStyle::Solid),
        path_effects: Some(with_stroke(3.0, "w")),
        ..GridOptions::default()
    };
    ax.grid_with(true, grid);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_patheffect2`: stroked contours and contour labels over an image.
fn patheffect2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let arr = Grid::from_fn(5, 5, |i, j| (i * 5 + j) as i64);
    let options = ImshowOptions {
        interpolation: Some(Interpolation::Nearest),
        ..ImshowOptions::default()
    };
    ax.imshow(ImageData::from(ScalarField::integers(&arr)), options)?;
    let z = arr.map(|&v| v as f64);
    let contours = ax.contour(
        &QuadField::new(z),
        ContourOptions {
            colors: Some(vec!["k".into()]),
            ..ContourOptions::default()
        },
    )?;
    contour_effects(&contours, &with_stroke(3.0, "w"));
    let labels = contours.clabel(ClabelOptions {
        fmt: ClabelFormat::Label("%2.0f".into()),
        use_clabeltext: true,
        ..ClabelOptions::default()
    })?;
    for label in labels {
        label.set_path_effects(with_stroke(3.0, "w"));
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_patheffect3`: a shadowed line, a stroked title, a legend with a
/// shadow, a text and its box with a stroke and a blue shadow, and text
/// drawn as two patch effects, the first hatched.
fn patheffect3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let line = ax.plot_with(
        [0.0, 1.0, 2.0, 3.0, 4.0],
        [1.0, 3.0, 5.0, 4.0, 3.0],
        LineProps::fmt("o-b")?.linewidth(4.0),
    )?;
    line.set_path_effects(vec![
        PathEffect::simple_line_shadow(LineShadow::default()),
        PathEffect::normal(),
    ]);
    ax.set_title(r"testing$^{123}$")
        .set_path_effects(with_stroke(1.0, "r"));
    let legend = ax.legend(LegendOptions {
        handles: Some(vec![LegendHandle::from(&line)]),
        labels: Some(vec![r"Line 1$^2$".into()]),
        fancybox: Some(true),
        loc: Some(LegendLoc::Anchored(AnchorLoc::UpperLeft)),
        ..LegendOptions::default()
    })?;
    legend
        .frame()
        .set_path_effects(vec![PathEffect::with_simple_patch_shadow(
            PatchShadow::default(),
        )]);

    let blue_shadow = PatchShadow {
        offset: (6.0, -3.0),
        ..PatchShadow::default()
    }
    .with_color("blue");
    let effects = vec![
        PathEffect::stroke(stroke(3.75, "k")),
        PathEffect::with_simple_patch_shadow(blue_shadow),
    ];
    let text = ax.text(2.0, 3.0, "Drop test");
    let bbox = TextBoxProps::new("circle,pad=0.1".parse()?).with_patch(
        PatchProps::default()
            .color("red")
            .path_effects(effects.clone()),
    );
    text.set_color("white").set_bbox(Some(bbox));
    text.set_path_effects(effects);

    hatch_shadow_text(&fig.text(0.02, 0.1, "Hatch shadow"))?;
    Ok(())
}

/// `test_patheffects_stroked_text`: white text outlined in black, in
/// every printable ASCII character.
fn patheffects_stroked_text(fig: &Figure) -> CaseResult {
    let chunks = [
        "A B C D E F G H I J K L",
        "M N O P Q R S T U V W",
        "X Y Z a b c d e f g h i j",
        "k l m n o p q r s t u v",
        "w x y z 0123456789",
        r"!@#$%^&*()-=_+[]\;'",
        ",./{}|:\"<>?",
    ];
    let font_size = 50.0;
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    for (i, chunk) in chunks.into_iter().enumerate() {
        ax.text(0.01, 0.9 - i as f64 * 0.13, chunk)
            .set_halign(HAlign::Left)
            .set_valign(VAlign::Center)
            .set_fontsize(font_size)
            .set_color("white")
            .set_path_effects(vec![
                PathEffect::stroke(stroke(font_size / 10.0, "black")),
                PathEffect::normal(),
            ]);
    }
    ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    ax.set_axis_off();
    Ok(())
}

/// `test_collection`: contours drawn as wide black patch outlines under a
/// stroke, labelled in white with a black stroke in blue sawtooth boxes.
fn collection(fig: &Figure) -> CaseResult {
    let (x, y) = (linspace(0.0, 10.0, 150), linspace(-5.0, 5.0, 100));
    let data = Grid::from_fn(100, 150, |i, j| x[j].sin() + y[i].cos());
    let ax = fig.subplot();
    let contours = ax.contour(&QuadField::new(data), ContourOptions::default())?;
    let outline = PatchEffect::new(
        (0.0, 0.0),
        PatchProps::default()
            .edgecolor("black")
            .facecolor("none")
            .linewidth(12.0),
    )?;
    contour_effects(
        &contours,
        &[
            PathEffect::path_patch(outline),
            PathEffect::stroke(Stroke::new(GcUpdate::default().linewidth(5.0))),
        ],
    );
    let labels = contours.clabel(ClabelOptions {
        colors: Some(vec!["white".into()]),
        ..ClabelOptions::default()
    })?;
    for label in labels {
        label.set_path_effects(with_stroke(3.0, "k"));
        let sawtooth = TextBoxProps::new("sawtooth".parse()?)
            .with_patch(PatchProps::default().facecolor("none").edgecolor("blue"));
        label.set_bbox(Some(sawtooth));
    }
    Ok(())
}

/// The zero contour of `g` on the survey grid of `test_tickedstroke`, in
/// `color` with ticked strokes.
fn ticked_contour(
    ax: &Axes,
    (x1, x2): (&Grid<f64>, &Grid<f64>),
    g: impl Fn(f64, f64) -> f64,
    color: &str,
    effects: &[PathEffect],
) -> CaseResult {
    let z = Grid::from_fn(x1.rows(), x1.cols(), |i, j| g(x1[(i, j)], x2[(i, j)]));
    let field = QuadField::new(z).with_coords(QuadCoords::Mesh {
        x: x1.clone(),
        y: x2.clone(),
    });
    let contours = ax.contour(
        &field,
        ContourOptions {
            levels: Levels::Values(vec![0.0]),
            colors: Some(vec![color.into()]),
            ..ContourOptions::default()
        },
    )?;
    contour_effects(&contours, effects);
    Ok(())
}

/// `test_tickedstroke`: ticked strokes on a circle, on lines (with a
/// legend) and on the zero contours of three constraints.
fn tickedstroke(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]);
    ax1.add_patch(PathShape(Path::unit_circle()))
        .update_props(PatchProps::default().facecolor("none").linewidth(2.0))
        .set_path_effects(ticked_stroke(-90.0, 10.0, 1.0));
    ax1.axis_mode(AxisMode::Equal)?;
    ax1.set_xlim(-2.0, 2.0)?.set_ylim(-2.0, 2.0)?;

    let default_length = TickedStroke::default().length;
    ax2.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("C0"))?
        .set_path_effects(ticked_stroke(135.0, 7.0, default_length));
    let x = linspace(0.0, 1.0, 101);
    let y: Vec<f64> = x.iter().map(|x| 0.3 * (x * 8.0).sin() + 0.4).collect();
    ax2.plot_with(&x, &y, LineProps::default().label("C1"))?
        .set_path_effects(vec![
            PathEffect::with_ticked_stroke(TickedStroke::default()),
        ]);
    ax2.legend(LegendOptions::default())?;

    let (x1, x2) = meshgrid(&linspace(0.001, 4.0, 101), &linspace(0.001, 4.0, 105));
    let survey = (&x1, &x2);
    ticked_contour(
        ax3,
        survey,
        |a, b| -(3.0 * a + b - 5.5),
        "k",
        &ticked_stroke(135.0, 10.0, default_length),
    )?;
    ticked_contour(
        ax3,
        survey,
        |a, b| -(a + 2.0 * b - 4.0),
        "r",
        &ticked_stroke(60.0, 10.0, 2.0),
    )?;
    ticked_contour(
        ax3,
        survey,
        |a, b| 0.8 + a.powi(-3) - b,
        "b",
        &ticked_stroke(45.0, 7.0, default_length),
    )?;
    ax3.set_xlim(0.0, 4.0)?.set_ylim(0.0, 4.0)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_patheffects_spaces_and_newlines`: blank and newline-led text in
/// boxes, drawn through the `Normal` effect.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn patheffects_spaces_and_newlines(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    for (y, text, color) in [
        (0.75, "         ", "salmon"),
        (0.25, "\nNewline also causes problems", "thistle"),
    ] {
        ax.text(0.5, y, text)
            .set_halign(HAlign::Center)
            .set_valign(VAlign::Center)
            .set_fontsize(20.0)
            .set_bbox(Some(
                TextBoxProps::default().with_patch(PatchProps::default().color(color)),
            ))
            .set_path_effects(vec![PathEffect::normal()]);
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_simple_line_shadow`'s test figure: a wide line drawn only as its
/// blue shadow, not offset.
fn simple_line_shadow(fig: &Figure) -> CaseResult {
    let x = linspace(-5.0, 5.0, 500);
    let y: Vec<f64> = x.iter().map(|x| (-x * x).exp()).collect();
    let shadow = LineShadow {
        offset: (0.0, 0.0),
        color: Some("blue".into()),
        ..LineShadow::default()
    };
    fig.subplot()
        .plot_with(x, y, LineProps::default().linewidth(5.0))?
        .set_path_effects(vec![PathEffect::simple_line_shadow(shadow)]);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_rendering/patheffect1", patheffect1),
    FigureCase::new("mpl_rendering/patheffect2", patheffect2),
    FigureCase::new("mpl_rendering/patheffect3", patheffect3).sized(8.0, 6.0, 100.0),
    FigureCase::new(
        "mpl_rendering/patheffects_stroked_text",
        patheffects_stroked_text,
    )
    .sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_rendering/collection", collection),
    FigureCase::new("mpl_rendering/tickedstroke", tickedstroke).sized(12.0, 4.0, 100.0),
    FigureCase::new(
        "mpl_rendering/patheffects_spaces_and_newlines",
        patheffects_spaces_and_newlines,
    ),
    classic("mpl_rendering/simple_line_shadow", simple_line_shadow),
];
