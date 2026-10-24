//! The pie charts of the section: a bar of pie, nested pies, a labelled
//! pie and donut, the features of `pie` and the labels of `pie_label`.

use plt::artist::ArtistHandle;
use plt::axes::{
    BarLabelOptions, BarLabelType, PieContainer, PieLabelOptions, PieOptions, PieShadow,
};
use plt::colors::color_sequence;
use plt::coords::Coords;
use plt::fonts::{FontSize, FontWeight, RelativeSize};
use plt::gridspec::SubplotOverrides;
use plt::legend::{BboxToAnchor, LegendOptions};
use plt::patches::{ConnectionEnd, ConnectionPatch};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, ArrowProps, HAlign, VAlign};

use super::strings;
use crate::figures::cases::common::{arrow, styled_box};
use crate::figures::{CaseResult, FigureCase};
use plt::offsetbox::AnchorLoc;
use plt::ticker::LabelFormat;

/// `autopct=format`.
fn percentages(format: &str) -> LabelFormat {
    LabelFormat::Template(format.into())
}

fn bar_of_pie(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.0),
        ..SubplotOverrides::default()
    })?;
    let overall_ratios = [0.27, 0.56, 0.17];
    let options = PieOptions {
        autopct: Some(percentages("%1.1f%%")),
        startangle: -180.0 * overall_ratios[0],
        labels: strings(&["Approve", "Disapprove", "Undecided"]),
        explode: vec![0.1, 0.0, 0.0],
        ..PieOptions::default()
    };
    let pie = ax1.pie(&overall_ratios, options)?;
    let age_ratios = [0.33, 0.54, 0.07, 0.06];
    let age_labels = ["Under 35", "35-49", "50-65", "Over 65"];
    let mut bottom = 1.0;
    let width = 0.2;
    for (j, (height, label)) in age_ratios.iter().zip(age_labels).rev().enumerate() {
        bottom -= height;
        let options = BarOptions {
            thickness: vec![width],
            base: vec![bottom],
            color: vec!["C0".into()],
            label: Some(label.into()),
            props: PatchProps::default().alpha(0.1 + 0.25 * j as f64),
            ..BarOptions::default()
        };
        let bc = ax2.bar([0.0], [*height], options)?;
        let centred = BarLabelOptions {
            labels: vec![format!("{:.0}%", height * 100.0)],
            label_type: BarLabelType::Center,
            ..BarLabelOptions::default()
        };
        ax2.bar_label(&bc, centred)?;
    }
    ax2.set_title("Age of approvers");
    ax2.legend(LegendOptions::default())?;
    ax2.set_axis_off();
    ax2.set_xlim(-2.5 * width, 2.5 * width)?;
    let wedge = pie.wedges[0].shape();
    let bar_height: f64 = age_ratios.iter().sum();
    for (theta, bar_y) in [(wedge.theta2, bar_height), (wedge.theta1, 0.0)] {
        let x = wedge.r * (std::f64::consts::PI / 180.0 * theta).cos() + wedge.center.x;
        let y = wedge.r * (std::f64::consts::PI / 180.0 * theta).sin() + wedge.center.y;
        let con = ConnectionPatch::new(
            ConnectionEnd::new(
                (-width / 2.0, bar_y),
                Coords::Transform(ax2.trans_data()),
                None,
            ),
            ConnectionEnd::new((x, y), Coords::Transform(ax1.trans_data()), None),
        );
        ax2.add_artist(con)
            .set_color((0.0, 0.0, 0.0))
            .set_linewidth(4.0);
    }
    Ok(())
}

/// `nested_pie`'s values: three groups of two (`NESTED_VALS`).
const NESTED_VALS: [[f64; 2]; 3] = [[60.0, 32.0], [37.0, 40.0], [29.0, 10.0]];

/// The width of each ring (`RING`).
const RING: f64 = 0.3;

fn nested_pie_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let tab20c = color_sequence("tab20c").ok_or("no tab20c colour sequence")?;
    let pick =
        |indices: &[usize]| -> Vec<Color> { indices.iter().map(|&i| tab20c[i].into()).collect() };
    let ring = |radius: f64, colors: Vec<Color>| PieOptions {
        radius,
        colors,
        wedge_width: Some(RING),
        wedgeprops: PatchProps::default().edgecolor("w"),
        ..PieOptions::default()
    };
    let sums = NESTED_VALS.map(|[a, b]| a + b);
    ax.pie(&sums, ring(1.0, pick(&[0, 4, 8])))?;
    ax.pie(
        NESTED_VALS.as_flattened(),
        ring(1.0 - RING, pick(&[1, 2, 5, 6, 9, 10])),
    )?;
    ax.set_aspect(Aspect::Equal)?;
    ax.set_title("Pie plot with `ax.pie`");
    Ok(())
}

/// The recipe of the labelled pie.
const PIE_RECIPE: [&str; 4] = ["375 g flour", "75 g sugar", "250 g butter", "300 g berries"];

fn pie_and_donut_labels_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let data = PIE_RECIPE
        .iter()
        .map(|item| {
            item.split_whitespace()
                .next()
                .unwrap_or_default()
                .parse::<f64>()
        })
        .collect::<Result<Vec<f64>, _>>()?;
    let ingredients: Vec<&str> = PIE_RECIPE
        .iter()
        .filter_map(|item| item.split_whitespace().last())
        .collect();
    let mut pie = ax.pie(&data, PieOptions::default())?;
    let white = PieLabelOptions {
        color: Some("w".into()),
        size: Some(FontSize::Points(8.0)),
        ..PieLabelOptions::default()
    };
    for text in ax.pie_label(&mut pie, "{frac:.1%}\n({absval:d}g)", white)? {
        text.set_weight(FontWeight::BOLD);
    }
    ax.legend(
        LegendOptions::default()
            .handles(&pie.wedges)
            .labels(ingredients)
            .title("Ingredients")
            .loc(AnchorLoc::CenterLeft)
            .bbox_to_anchor(BboxToAnchor::Bounds(1.0, 0.0, 0.5, 1.0)),
    )?;
    ax.set_title("Matplotlib bakery: A pie");
    Ok(())
}

fn pie_and_donut_labels_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_aspect(Aspect::Equal)?;
    let recipe = [
        "225 g flour",
        "90 g sugar",
        "1 egg",
        "60 g butter",
        "100 ml milk",
        "1/2 package of yeast",
    ];
    let donut = PieOptions {
        wedge_width: Some(0.5),
        startangle: -40.0,
        ..PieOptions::default()
    };
    let pie = ax.pie(&[225.0, 90.0, 50.0, 60.0, 100.0, 5.0], donut)?;
    let bbox = || {
        styled_box(
            "square,pad=0.3",
            PatchProps::default()
                .facecolor("w")
                .edgecolor("k")
                .linewidth(0.72),
        )
    };
    for (wedge, item) in pie.wedges.iter().zip(recipe) {
        let wedge = wedge.shape();
        let ang = (wedge.theta2 - wedge.theta1) / 2.0 + wedge.theta1;
        let (y, x) = ang.to_radians().sin_cos();
        let side = x.signum();
        let halign = if side < 0.0 {
            HAlign::Right
        } else {
            HAlign::Left
        };
        let options = AnnotateOptions {
            xytext: Some((1.35 * side, 1.4 * y).into()),
            arrowprops: Some(ArrowProps::Fancy(arrow(
                "-",
                &format!("angle,angleA=0,angleB={ang}"),
            )?)),
            ..AnnotateOptions::default()
        };
        ax.annotate(item, (x, y), options)?
            .set_halign(halign)
            .set_bbox(bbox()?)
            .set_zorder(0.0)
            .set_valign(VAlign::Center);
    }
    ax.set_title("Matplotlib bakery: A donut");
    Ok(())
}

/// The sizes and labels of `pie_features`.
const PIE_SIZES: [f64; 4] = [15.0, 30.0, 45.0, 10.0];
const PIE_LABELS: [&str; 4] = ["Frogs", "Hogs", "Dogs", "Logs"];

/// Explodes the second slice, 'Hogs' (`HOGS`).
const HOGS: [f64; 4] = [0.0, 0.1, 0.0, 0.0];

/// A pie of [`PIE_SIZES`] labelled with [`PIE_LABELS`] and drawn with
/// `options` on top.
fn feature_pie(fig: &Figure, options: PieOptions) -> CaseResult {
    let options = PieOptions {
        labels: strings(&PIE_LABELS),
        ..options
    };
    fig.subplot().pie(&PIE_SIZES, options)?;
    Ok(())
}

fn pie_features_1(fig: &Figure) -> CaseResult {
    feature_pie(fig, PieOptions::default())
}

fn pie_features_2(fig: &Figure) -> CaseResult {
    feature_pie(
        fig,
        PieOptions {
            autopct: Some(percentages("%1.1f%%")),
            ..PieOptions::default()
        },
    )
}

fn pie_features_3(fig: &Figure) -> CaseResult {
    let colors = ["olivedrab", "rosybrown", "gray", "saddlebrown"];
    feature_pie(
        fig,
        PieOptions {
            colors: colors.map(Color::from).to_vec(),
            ..PieOptions::default()
        },
    )
}

fn pie_features_4(fig: &Figure) -> CaseResult {
    feature_pie(
        fig,
        PieOptions {
            hatches: strings(&["**O", "oO", "O.O", ".||."]),
            ..PieOptions::default()
        },
    )
}

fn pie_features_5(fig: &Figure) -> CaseResult {
    feature_pie(
        fig,
        PieOptions {
            autopct: Some(percentages("%1.1f%%")),
            pctdistance: 1.25,
            labeldistance: Some(0.6),
            ..PieOptions::default()
        },
    )
}

/// The exploded pie with percentages, turned to start at the top, under
/// `shadow`.
fn shadowed(shadow: PieShadow) -> PieOptions {
    PieOptions {
        explode: HOGS.to_vec(),
        autopct: Some(percentages("%1.1f%%")),
        shadow: Some(shadow),
        startangle: 90.0,
        ..PieOptions::default()
    }
}

fn pie_features_6(fig: &Figure) -> CaseResult {
    feature_pie(fig, shadowed(PieShadow::default()))
}

fn pie_features_7(fig: &Figure) -> CaseResult {
    feature_pie(
        fig,
        PieOptions {
            autopct: Some(percentages("%.0f%%")),
            text_size: Some(FontSize::Relative(RelativeSize::Small)),
            radius: 0.5,
            ..PieOptions::default()
        },
    )
}

fn pie_features_8(fig: &Figure) -> CaseResult {
    let shadow = PieShadow {
        ox: -0.04,
        shade: 0.9,
        props: PatchProps::default().edgecolor("none"),
        ..PieShadow::default()
    };
    feature_pie(fig, shadowed(shadow))
}

/// The data and labels of `pie_label`.
const PIE_DATA: [f64; 4] = [36.0, 24.0, 8.0, 12.0];
const BREAKFAST: &[&str] = &["spam", "eggs", "bacon", "sausage"];

/// How one call of `pie_label` labels the wedges.
enum Labelling {
    /// The breakfast names.
    Names(PieLabelOptions),
    /// A template of the wedges' values and fractions.
    Template(&'static str, PieLabelOptions),
}

/// Labels at `distance` radii.
fn at(distance: f64) -> PieLabelOptions {
    PieLabelOptions {
        distance,
        ..PieLabelOptions::default()
    }
}

/// A pie of [`PIE_DATA`] labelled once per labelling (`labelled_pie`).
fn labelled_pie(fig: &Figure, labellings: Vec<Labelling>) -> CaseResult {
    let ax = fig.subplot();
    let mut pie: PieContainer = ax.pie(&PIE_DATA, PieOptions::default())?;
    for labelling in labellings {
        match labelling {
            Labelling::Names(options) => ax.pie_label(&mut pie, BREAKFAST, options)?,
            Labelling::Template(template, options) => ax.pie_label(&mut pie, template, options)?,
        };
    }
    Ok(())
}

fn pie_label_1(fig: &Figure) -> CaseResult {
    labelled_pie(fig, vec![Labelling::Names(PieLabelOptions::default())])
}

fn pie_label_2(fig: &Figure) -> CaseResult {
    labelled_pie(fig, vec![Labelling::Names(at(1.1))])
}

fn pie_label_3(fig: &Figure) -> CaseResult {
    let rotated = PieLabelOptions {
        rotate: true,
        ..PieLabelOptions::default()
    };
    labelled_pie(fig, vec![Labelling::Names(rotated)])
}

fn pie_label_4(fig: &Figure) -> CaseResult {
    labelled_pie(
        fig,
        vec![Labelling::Template(
            "{absval:.1f}",
            PieLabelOptions::default(),
        )],
    )
}

fn pie_label_5(fig: &Figure) -> CaseResult {
    labelled_pie(
        fig,
        vec![Labelling::Template(
            "{frac:.1%}",
            PieLabelOptions::default(),
        )],
    )
}

fn pie_label_6(fig: &Figure) -> CaseResult {
    labelled_pie(
        fig,
        vec![Labelling::Template(
            "{absval:d}\n{frac:.1%}",
            PieLabelOptions::default(),
        )],
    )
}

fn pie_label_7(fig: &Figure) -> CaseResult {
    let large_white = PieLabelOptions {
        size: Some(FontSize::Relative(RelativeSize::Large)),
        color: Some("white".into()),
        ..PieLabelOptions::default()
    };
    labelled_pie(fig, vec![Labelling::Names(large_white)])
}

fn pie_label_8(fig: &Figure) -> CaseResult {
    labelled_pie(
        fig,
        vec![
            Labelling::Names(at(1.1)),
            Labelling::Template("{frac:.1%}", at(0.7)),
            Labelling::Template("{absval:d}", at(0.4)),
        ],
    )
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/bar_of_pie", bar_of_pie).sized(9.0, 5.0, 100.0),
    FigureCase::new("gallery_stats/nested_pie_1", nested_pie_1),
    FigureCase::new(
        "gallery_stats/pie_and_donut_labels_1",
        pie_and_donut_labels_1,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new(
        "gallery_stats/pie_and_donut_labels_2",
        pie_and_donut_labels_2,
    )
    .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_stats/pie_features_1", pie_features_1),
    FigureCase::new("gallery_stats/pie_features_2", pie_features_2),
    FigureCase::new("gallery_stats/pie_features_3", pie_features_3),
    FigureCase::new("gallery_stats/pie_features_4", pie_features_4),
    FigureCase::new("gallery_stats/pie_features_5", pie_features_5),
    FigureCase::new("gallery_stats/pie_features_6", pie_features_6),
    FigureCase::new("gallery_stats/pie_features_7", pie_features_7),
    FigureCase::new("gallery_stats/pie_features_8", pie_features_8),
    FigureCase::new("gallery_stats/pie_label_1", pie_label_1),
    FigureCase::new("gallery_stats/pie_label_2", pie_label_2),
    FigureCase::new("gallery_stats/pie_label_3", pie_label_3),
    FigureCase::new("gallery_stats/pie_label_4", pie_label_4),
    FigureCase::new("gallery_stats/pie_label_5", pie_label_5),
    FigureCase::new("gallery_stats/pie_label_6", pie_label_6),
    FigureCase::new("gallery_stats/pie_label_7", pie_label_7),
    FigureCase::new("gallery_stats/pie_label_8", pie_label_8),
];
