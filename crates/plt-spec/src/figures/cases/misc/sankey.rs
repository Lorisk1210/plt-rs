//! Sankey cases of `figures/cases_axes_misc.py`.

use plt::axes::Axes;
use plt::figure::Figure;
use plt::patches::PatchProps;
use plt::sankey::{
    FlowOrientation, PathLengths, QuantityFormat, Sankey, SankeyOptions, Subdiagram,
};

use crate::figures::FigureCase;

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
];
