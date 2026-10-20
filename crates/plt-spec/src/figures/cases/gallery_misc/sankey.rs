//! Sankey diagrams: the basics, a long chain of links and a power cycle
//! (`sankey_basics`, `sankey_links` and `sankey_rankine`).

use plt::fonts::FontWeight;
use plt::prelude::*;
use plt::sankey::{PathLengths, QuantityFormat, Sankey, SankeyOptions, Subdiagram};

use crate::figures::cases::common::{flow_labels, flow_orientations, shown_labels, tickless};
use crate::figures::{CaseResult, FigureCase};

/// The axes `Sankey` makes when given none, with an optional title.
fn sankey_axes(fig: &Figure, title: &str) -> Axes {
    let ax = fig.subplot();
    tickless(&ax);
    if !title.is_empty() {
        ax.set_title(title);
    }
    ax
}

fn sankey_basics_1(fig: &Figure) -> CaseResult {
    let ax = sankey_axes(fig, "");
    let flows = Subdiagram {
        flows: vec![0.25, 0.15, 0.60, -0.20, -0.15, -0.05, -0.50, -0.10],
        labels: shown_labels(&["", "", "", "First", "Second", "Third", "Fourth", "Fifth"]),
        orientations: flow_orientations(&[-1, 1, 0, 1, 1, 1, 0, -1]),
        ..Subdiagram::default()
    };
    Sankey::new(&ax, SankeyOptions::default())?
        .add(flows)?
        .finish();
    ax.set_title("The default settings produce a diagram like this.");
    Ok(())
}

fn sankey_basics_2(fig: &Figure) -> CaseResult {
    let ax = sankey_axes(fig, "Flow Diagram of a Widget");
    let options = SankeyOptions {
        scale: 0.01,
        offset: 0.2,
        head_angle: 180.0,
        format: QuantityFormat::Printf("%.0f".into()),
        unit: Some("%".into()),
        ..SankeyOptions::default()
    };
    let widget = Subdiagram {
        flows: vec![25.0, 0.0, 60.0, -10.0, -20.0, -5.0, -15.0, -10.0, -40.0],
        labels: shown_labels(&[
            "", "", "", "First", "Second", "Third", "Fourth", "Fifth", "Hurray!",
        ]),
        orientations: flow_orientations(&[-1, 1, 0, 1, 1, 1, -1, -1, 0]),
        pathlengths: PathLengths::PerFlow(vec![
            0.25, 0.25, 0.25, 0.25, 0.25, 0.6, 0.25, 0.25, 0.25,
        ]),
        patchlabel: "Widget\nA".into(),
        ..Subdiagram::default()
    };
    let mut sankey = Sankey::new(&ax, options)?;
    sankey.add(widget)?;
    let diagram = sankey.finish().first().ok_or("one diagram")?;
    diagram.texts.last().ok_or("flow labels")?.set_color("r");
    diagram.text.set_weight(FontWeight::BOLD);
    Ok(())
}

fn sankey_basics_3(fig: &Figure) -> CaseResult {
    let ax = sankey_axes(fig, "Two Systems");
    let options = SankeyOptions {
        unit: None,
        ..SankeyOptions::default()
    };
    let one = Subdiagram {
        flows: vec![0.25, 0.15, 0.60, -0.10, -0.05, -0.25, -0.15, -0.10, -0.35],
        orientations: flow_orientations(&[-1, 1, 0, 1, 1, 1, -1, -1, 0]),
        props: PatchProps::default().label("one"),
        ..Subdiagram::default()
    };
    let two = Subdiagram {
        flows: vec![-0.25, 0.15, 0.1],
        orientations: flow_orientations(&[-1, -1, -1]),
        prior: Some(0),
        connect: (0, 0),
        props: PatchProps::default().label("two"),
        ..Subdiagram::default()
    };
    let mut sankey = Sankey::new(&ax, options)?;
    sankey.add(one)?.add(two)?;
    let diagrams = sankey.finish();
    diagrams.last().ok_or("two diagrams")?.patch.set_hatch("/");
    ax.legend(plt::legend::LegendOptions::default())?;
    Ok(())
}

/// A link of `sankey_links`: one flow in and out, turned by
/// `orientations`, half transparent.
fn link(orientations: &[i8], patchlabel: String, prior: Option<usize>) -> Subdiagram {
    Subdiagram {
        flows: vec![1.0, -1.0],
        orientations: flow_orientations(orientations),
        patchlabel,
        prior,
        connect: (1, 0),
        props: PatchProps::default().alpha(0.5),
        ..Subdiagram::default()
    }
}

/// `side(sankey, n)`: `n` pairs of links zigzagging along a side.
fn side(sankey: &mut Sankey, n: usize) -> CaseResult {
    let prior = sankey.diagrams().len();
    for i in (0..2 * n).step_by(2) {
        sankey.add(link(
            &[-1, -1],
            (prior + i).to_string(),
            Some(prior + i - 1),
        ))?;
        sankey.add(link(&[1, 1], (prior + i + 1).to_string(), Some(prior + i)))?;
    }
    Ok(())
}

/// `corner(sankey)`: a black link turning the chain.
fn corner(sankey: &mut Sankey) -> CaseResult {
    let prior = sankey.diagrams().len();
    let mut turn = link(&[0, 1], prior.to_string(), Some(prior - 1));
    turn.props = turn.props.facecolor("k");
    sankey.add(turn)?;
    Ok(())
}

fn sankey_links(fig: &Figure) -> CaseResult {
    let ax = sankey_axes(fig, "Why would you want to do this?\n(But you could.)");
    let options = SankeyOptions {
        unit: None,
        ..SankeyOptions::default()
    };
    let mut sankey = Sankey::new(&ax, options)?;
    let mut first = link(&[0, 1], "0".into(), None);
    first.props = PatchProps::default().facecolor("k");
    first.rotation = 45.0;
    sankey.add(first)?;
    side(&mut sankey, 6)?;
    for _ in 0..3 {
        corner(&mut sankey)?;
        side(&mut sankey, 6)?;
    }
    sankey.finish();
    Ok(())
}

/// The heat flows of the Rankine cycle in MW (`Hdot`).
const HDOT: [f64; 19] = [
    260.431, 35.078, 180.794, 221.115, 22.700, 142.361, 10.193, 10.210, 43.670, 44.312, 68.631,
    10.758, 10.758, 0.017, 0.642, 232.121, 44.559, 100.613, 132.168,
];

/// How long the paths of a component are (`pathlengths`).
#[derive(Clone, Copy)]
enum Lengths {
    /// One length per flow.
    Each(&'static [f64]),
    /// The length of the innermost top and bottom paths.
    Inner(f64),
}

/// One component of the Rankine cycle: its flows (`sign * HDOT[i]`),
/// labels, path lengths, orientations, colour and connection.
struct Component {
    patchlabel: &'static str,
    flows: &'static [(usize, f64)],
    labels: &'static [Option<&'static str>],
    pathlengths: Lengths,
    orientations: &'static [i8],
    color: &'static str,
    prior: Option<usize>,
    connect: (usize, usize),
    trunklength: Option<f64>,
    rotation: f64,
}

impl Component {
    fn subdiagram(&self) -> Subdiagram {
        Subdiagram {
            patchlabel: self.patchlabel.into(),
            flows: self.flows.iter().map(|&(i, sign)| sign * HDOT[i]).collect(),
            labels: flow_labels(self.labels),
            pathlengths: match self.pathlengths {
                Lengths::Each(lengths) => PathLengths::PerFlow(lengths.to_vec()),
                Lengths::Inner(length) => PathLengths::Inner(length),
            },
            orientations: flow_orientations(self.orientations),
            prior: self.prior,
            connect: self.connect,
            trunklength: self
                .trunklength
                .unwrap_or_else(|| Subdiagram::default().trunklength),
            rotation: self.rotation,
            props: PatchProps::default().facecolor(self.color),
        }
    }
}

/// The colours of the cycle's components.
const GREEN: &str = "#37c959";
const RED: &str = "#ff5555";
const BLUE: &str = "#58b1fa";

/// The components of the Rankine cycle, each connected to an earlier one.
const RANKINE: [Component; 10] = [
    Component {
        patchlabel: "\n\nPump 1",
        flows: &[(13, 1.0), (6, 1.0), (7, -1.0)],
        labels: &[Some("Shaft power"), Some(""), None],
        pathlengths: Lengths::Each(&[0.4, 0.883, 0.25]),
        orientations: &[1, -1, 0],
        color: GREEN,
        prior: None,
        connect: (0, 0),
        trunklength: None,
        rotation: 90.0,
    },
    Component {
        patchlabel: "\n\nOpen\nheater",
        flows: &[(11, 1.0), (7, 1.0), (4, 1.0), (8, -1.0)],
        labels: &[None, Some(""), None, None],
        pathlengths: Lengths::Each(&[0.25, 0.25, 1.93, 0.25]),
        orientations: &[1, 0, -1, 0],
        color: GREEN,
        prior: Some(0),
        connect: (2, 1),
        trunklength: None,
        rotation: 0.0,
    },
    Component {
        patchlabel: "\n\nPump 2",
        flows: &[(14, 1.0), (8, 1.0), (9, -1.0)],
        labels: &[Some("Shaft power"), Some(""), None],
        pathlengths: Lengths::Each(&[0.4, 0.25, 0.25]),
        orientations: &[1, 0, 0],
        color: GREEN,
        prior: Some(1),
        connect: (3, 1),
        trunklength: None,
        rotation: 0.0,
    },
    Component {
        patchlabel: "Closed\nheater",
        flows: &[(9, 1.0), (1, 1.0), (11, -1.0), (10, -1.0)],
        labels: &[Some(""), Some(""), None, None],
        pathlengths: Lengths::Each(&[0.25, 1.543, 0.25, 0.25]),
        orientations: &[0, -1, 1, -1],
        color: GREEN,
        prior: Some(2),
        connect: (2, 0),
        trunklength: Some(2.914),
        rotation: 0.0,
    },
    Component {
        patchlabel: "Trap",
        flows: &[(11, 1.0), (12, -1.0)],
        labels: &[Some("\n"), None],
        pathlengths: Lengths::Each(&[1.0, 1.01]),
        orientations: &[1, 1],
        color: GREEN,
        prior: Some(3),
        connect: (2, 0),
        trunklength: Some(5.102),
        rotation: 0.0,
    },
    Component {
        patchlabel: "Steam\ngenerator",
        flows: &[(15, 1.0), (10, 1.0), (2, 1.0), (3, -1.0), (0, -1.0)],
        labels: &[Some("Heat rate"), Some(""), Some(""), None, None],
        pathlengths: Lengths::Inner(0.25),
        orientations: &[1, 0, -1, -1, -1],
        color: RED,
        prior: Some(3),
        connect: (3, 1),
        trunklength: None,
        rotation: 0.0,
    },
    Component {
        patchlabel: "\n\n\nTurbine 1",
        flows: &[(0, 1.0), (16, -1.0), (1, -1.0), (2, -1.0)],
        labels: &[Some(""), None, None, None],
        pathlengths: Lengths::Each(&[0.25, 0.153, 1.543, 0.25]),
        orientations: &[0, 1, -1, -1],
        color: GREEN,
        prior: Some(5),
        connect: (4, 0),
        trunklength: None,
        rotation: 0.0,
    },
    Component {
        patchlabel: "\n\n\nReheat",
        flows: &[(2, 1.0), (2, -1.0)],
        labels: &[None, None],
        pathlengths: Lengths::Each(&[0.725, 0.25]),
        orientations: &[-1, 0],
        color: GREEN,
        prior: Some(6),
        connect: (3, 0),
        trunklength: None,
        rotation: 0.0,
    },
    Component {
        patchlabel: "Turbine 2",
        flows: &[(3, 1.0), (16, 1.0), (5, -1.0), (4, -1.0), (17, -1.0)],
        labels: &[
            None,
            Some("Shaft power"),
            None,
            Some(""),
            Some("Shaft power"),
        ],
        pathlengths: Lengths::Each(&[0.751, 0.15, 0.25, 1.93, 0.25]),
        orientations: &[0, -1, 0, -1, 1],
        color: GREEN,
        prior: Some(6),
        connect: (1, 1),
        trunklength: Some(3.212),
        rotation: 0.0,
    },
    Component {
        patchlabel: "Condenser",
        flows: &[(5, 1.0), (18, -1.0), (6, -1.0)],
        labels: &[Some(""), Some("Heat rate"), None],
        pathlengths: Lengths::Each(&[0.45, 0.25, 0.883]),
        orientations: &[-1, 1, 0],
        color: BLUE,
        prior: Some(8),
        connect: (2, 0),
        trunklength: Some(1.764),
        rotation: 0.0,
    },
];

fn sankey_rankine(fig: &Figure) -> CaseResult {
    let ax = sankey_axes(
        fig,
        "Rankine Power Cycle: Example 8.6 from Moran and Shapiro\n\"Fundamentals of \
         Engineering Thermodynamics \", 6th ed., 2008",
    );
    let options = SankeyOptions {
        format: QuantityFormat::Printf("%.3G".into()),
        unit: Some(" MW".into()),
        gap: 0.5,
        scale: 1.0 / HDOT[0],
        ..SankeyOptions::default()
    };
    let mut sankey = Sankey::new(&ax, options)?;
    for component in &RANKINE {
        sankey.add(component.subdiagram())?;
    }
    for diagram in sankey.finish() {
        diagram.text.set_weight(FontWeight::BOLD).set_fontsize(10.0);
        for text in &diagram.texts {
            text.set_fontsize(10.0);
        }
    }
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/sankey_basics_1", sankey_basics_1),
    FigureCase::new("gallery_misc/sankey_basics_2", sankey_basics_2),
    FigureCase::new("gallery_misc/sankey_basics_3", sankey_basics_3),
    FigureCase::new("gallery_misc/sankey_links", sankey_links),
    FigureCase::new("gallery_misc/sankey_rankine", sankey_rankine).sized(8.0, 9.0, 100.0),
];
