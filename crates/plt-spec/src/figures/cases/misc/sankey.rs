//! Sankey cases of `figures/cases_axes_misc.py`.

use std::rc::Rc;

use plt::axes::Axes;
use plt::figure::Figure;
use plt::fonts::FontWeight;
use plt::patches::PatchProps;
use plt::sankey::{
    FlowOrientation, PathLengths, QuantityFormat, Sankey, SankeyOptions, Subdiagram,
};

use crate::figures::{CaseResult, FigureCase};

/// An axes without ticks, as both cases start.
fn bare_axes(fig: &Figure) -> Axes {
    let ax = fig.subplot();
    ax.set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    ax
}

/// matplotlib's orientation codes `-1`, `0` and `1`.
fn orientations(codes: &[i8]) -> Vec<FlowOrientation> {
    codes
        .iter()
        .map(|code| match code {
            1 => FlowOrientation::Top,
            -1 => FlowOrientation::Bottom,
            _ => FlowOrientation::Side,
        })
        .collect()
}

/// Labels, `None` for Python's `None`.
fn labels(texts: &[Option<&str>]) -> Vec<Option<String>> {
    texts.iter().map(|t| t.map(str::to_owned)).collect()
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/sankey_simple", |fig| {
        let ax = bare_axes(fig);
        let options = SankeyOptions {
            unit: None,
            ..SankeyOptions::default()
        };
        let flows = Subdiagram {
            flows: vec![0.25, 0.15, 0.60, -0.20, -0.15, -0.05, -0.50, -0.10],
            labels: labels(&[
                Some(""),
                Some(""),
                Some(""),
                Some("First"),
                Some("Second"),
                Some("Third"),
                Some("Fourth"),
                Some("Fifth"),
            ]),
            orientations: orientations(&[-1, 1, 0, 1, 1, 1, 0, -1]),
            ..Subdiagram::default()
        };
        Sankey::new(&ax, options)?.add(flows)?.finish();
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/sankey_connected", |fig| {
        let ax = bare_axes(fig);
        let options = SankeyOptions {
            scale: 0.01,
            offset: 0.2,
            format: QuantityFormat::Printf("%.0f".into()),
            unit: Some("%".into()),
            ..SankeyOptions::default()
        };
        let mut sankey = Sankey::new(&ax, options)?;
        let widget = Subdiagram {
            flows: vec![25.0, 0.0, 60.0, -10.0, -20.0, -5.0, -15.0, -10.0, -40.0],
            labels: labels(&[
                Some(""),
                Some(""),
                Some(""),
                Some("First"),
                Some("Second"),
                Some("Third"),
                Some("Fourth"),
                Some("Fifth"),
                Some("Hurray!"),
            ]),
            orientations: orientations(&[-1, 1, 0, 1, 1, 1, -1, -1, 0]),
            pathlengths: PathLengths::PerFlow(vec![
                0.25, 0.25, 0.25, 0.25, 0.25, 0.6, 0.25, 0.25, 0.25,
            ]),
            patchlabel: "Widget\nA".into(),
            props: PatchProps::default().facecolor("tab:blue").alpha(0.4),
            ..Subdiagram::default()
        };
        let outputs = Subdiagram {
            flows: vec![40.0, -20.0, -20.0],
            orientations: orientations(&[0, 1, -1]),
            prior: Some(0),
            connect: (8, 0),
            labels: labels(&[None, Some("out A"), Some("out B")]),
            props: PatchProps::default().facecolor("tab:orange").alpha(0.4),
            ..Subdiagram::default()
        };
        sankey.add(widget)?.add(outputs)?.finish();
        Ok(())
    })
    .sized(5.0, 4.0, 100.0),
    FigureCase::new("misc/sankey_rotation_shape_params", rotation_shape_params)
        .sized(5.0, 5.0, 100.0),
    FigureCase::new("misc/sankey_units_format_texts", units_format_texts).sized(6.0, 4.0, 100.0),
    FigureCase::new("misc/sankey_three_chained", three_chained).sized(6.0, 5.0, 100.0),
    FigureCase::new("misc/sankey_zero_and_unbalanced_flows", zero_and_unbalanced)
        .sized(5.0, 4.0, 100.0),
];

/// Labels that all show text.
fn shown(texts: &[&str]) -> Vec<Option<String>> {
    texts.iter().map(|&t| Some(t.to_owned())).collect()
}

fn rotation_shape_params(fig: &Figure) -> CaseResult {
    let ax = bare_axes(fig);
    let options = SankeyOptions {
        scale: 1.0 / 3.0,
        gap: 0.4,
        radius: 0.2,
        shoulder: 0.05,
        head_angle: 120.0,
        offset: 0.3,
        margin: 0.2,
        unit: None,
        ..SankeyOptions::default()
    };
    let rotated = Subdiagram {
        flows: vec![1.0, -0.4, -0.3, -0.3],
        orientations: orientations(&[0, 1, 0, -1]),
        rotation: 90.0,
        trunklength: 2.0,
        patchlabel: "rotated".into(),
        labels: shown(&["in", "a", "b", "c"]),
        props: PatchProps::default()
            .facecolor("tab:green")
            .edgecolor("k")
            .linewidth(1.5)
            .alpha(0.6),
        ..Subdiagram::default()
    };
    Sankey::new(&ax, options)?.add(rotated)?.finish();
    Ok(())
}

fn units_format_texts(fig: &Figure) -> CaseResult {
    let ax = bare_axes(fig);
    let options = SankeyOptions {
        scale: 0.02,
        unit: Some(" kW".into()),
        format: QuantityFormat::Printf("%.1f".into()),
        ..SankeyOptions::default()
    };
    let plant = Subdiagram {
        flows: vec![50.0, -20.0, -15.0, -10.0, -5.0],
        orientations: orientations(&[0, 1, 1, -1, -1]),
        labels: shown(&["supply", "heating", "lights", "pumps", "losses"]),
        pathlengths: PathLengths::PerFlow(vec![0.5, 0.25, 0.5, 0.25, 0.5]),
        patchlabel: "plant".into(),
        props: PatchProps::default().facecolor("lightsteelblue"),
        ..Subdiagram::default()
    };
    let mut sankey = Sankey::new(&ax, options)?;
    sankey.add(plant)?;
    let diagram = sankey.finish().first().ok_or("one diagram")?;
    diagram.text.set_weight(FontWeight::BOLD);
    for text in &diagram.texts {
        text.set_fontsize(8.0);
    }
    diagram.texts.last().ok_or("labels")?.set_color("tab:red");
    Ok(())
}

fn three_chained(fig: &Figure) -> CaseResult {
    let ax = bare_axes(fig);
    let options = SankeyOptions {
        unit: None,
        gap: 0.3,
        ..SankeyOptions::default()
    };
    let faded = |color: &str| PatchProps::default().facecolor(color).alpha(0.5);
    let first = Subdiagram {
        flows: vec![1.0, -0.6, -0.4],
        orientations: orientations(&[0, 0, 1]),
        labels: shown(&["in", "", "up"]),
        patchlabel: "A".into(),
        props: faded("tab:blue"),
        ..Subdiagram::default()
    };
    let second = Subdiagram {
        flows: vec![0.6, -0.2, -0.4],
        orientations: orientations(&[0, -1, 0]),
        prior: Some(0),
        connect: (1, 0),
        labels: labels(&[None, Some("down"), Some("")]),
        patchlabel: "B".into(),
        props: faded("tab:orange"),
        ..Subdiagram::default()
    };
    let third = Subdiagram {
        flows: vec![0.4, -0.1, -0.3],
        orientations: orientations(&[0, 1, 0]),
        prior: Some(1),
        connect: (2, 0),
        labels: labels(&[None, Some("leak"), Some("out")]),
        patchlabel: "C".into(),
        props: faded("tab:green"),
        trunklength: 0.5,
        ..Subdiagram::default()
    };
    Sankey::new(&ax, options)?
        .add(first)?
        .add(second)?
        .add(third)?
        .finish();
    Ok(())
}

fn zero_and_unbalanced(fig: &Figure) -> CaseResult {
    let ax = bare_axes(fig);
    let options = SankeyOptions {
        format: QuantityFormat::Func(Rc::new(|q| format!("{q:+.2}"))),
        tolerance: 0.01,
        ..SankeyOptions::default()
    };
    let flows = Subdiagram {
        flows: vec![0.7, 0.3, 0.0, -0.5, -0.3, 0.005],
        orientations: orientations(&[0, 1, -1, 0, -1, 1]),
        labels: shown(&["main", "side", "zero", "out", "drain", "tiny"]),
        pathlengths: PathLengths::Inner(0.4),
        props: PatchProps::default()
            .facecolor("wheat")
            .edgecolor("saddlebrown"),
        ..Subdiagram::default()
    };
    Sankey::new(&ax, options)?.add(flows)?.finish();
    Ok(())
}
