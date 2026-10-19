use super::*;
use crate::figure::Figure;

fn sankey(options: SankeyOptions) -> (Axes, Sankey) {
    let ax = Figure::new().subplot();
    let sankey = Sankey::new(&ax, options).unwrap();
    (ax, sankey)
}

#[test]
fn settings_are_validated() {
    let ax = Figure::new().subplot();
    let with = |options: SankeyOptions| Sankey::new(&ax, options).err();
    assert_eq!(
        with(SankeyOptions {
            gap: -1.0,
            ..SankeyOptions::default()
        }),
        Some(SankeyError::NegativeGap)
    );
    assert_eq!(
        with(SankeyOptions {
            radius: 0.5,
            ..SankeyOptions::default()
        }),
        Some(SankeyError::RadiusAboveGap)
    );
    assert_eq!(
        with(SankeyOptions {
            head_angle: -1.0,
            ..SankeyOptions::default()
        }),
        Some(SankeyError::NegativeHeadAngle)
    );
}

#[test]
fn inner_lengths_nest_outwards() {
    let directions = [
        Some(Direction::Down),
        Some(Direction::Down),
        Some(Direction::Right),
    ];
    let inputs = [Some(true), Some(true), Some(true)];
    let lengths = default_lengths(0.25, &directions, &inputs, &[0.5, 0.25, 0.25]);
    assert_eq!(lengths, [0.25, 0.75, 0.0]);
}

#[test]
fn labels_carry_quantities_unless_hidden() {
    let (_, mut chart) = sankey(SankeyOptions {
        format: QuantityFormat::Printf("%.0f".into()),
        unit: Some("%".into()),
        ..SankeyOptions::default()
    });
    let sub = Subdiagram {
        flows: vec![25.0, 0.0, -25.0],
        labels: vec![Some("in".into()), Some("zero".into()), Some(String::new())],
        ..Subdiagram::default()
    };
    let diagram = &chart.add(sub).unwrap().diagrams()[0];
    let texts: Vec<String> = diagram.texts.iter().map(Text::text).collect();
    assert_eq!(texts, ["in\n25%", "", "25%"]);
    assert_eq!(diagram.angles, [Some(0.0), None, Some(0.0)]);
    assert_eq!(diagram.tips[1], Point::ORIGIN);
    let (_, mut hidden) = sankey(SankeyOptions {
        unit: None,
        ..SankeyOptions::default()
    });
    let labelled = Subdiagram {
        labels: vec![Some("a".into())],
        ..Subdiagram::default()
    };
    let diagram = &hidden.add(labelled).unwrap().diagrams()[0];
    assert_eq!(diagram.texts[0].text(), "a");
}

#[test]
fn connected_diagrams_meet_at_their_tips() {
    let (ax, mut chart) = sankey(SankeyOptions::default());
    let first = Subdiagram {
        flows: vec![1.0, -0.5, -0.5],
        orientations: vec![
            FlowOrientation::Side,
            FlowOrientation::Top,
            FlowOrientation::Side,
        ],
        ..Subdiagram::default()
    };
    let second = Subdiagram {
        flows: vec![0.5, -0.5],
        prior: Some(0),
        connect: (2, 0),
        ..Subdiagram::default()
    };
    let diagrams = chart.add(first).unwrap().add(second).unwrap().finish();
    let (a, b) = (diagrams[0].tips[2], diagrams[1].tips[0]);
    assert!((a - b).length() < 1e-12);
    assert_eq!(diagrams[1].angles[0], Some(0.0));
    let (x0, x1) = ax.xlim();
    assert!(x0 <= -0.25 - 0.4 && x1 >= b.x + 0.4);
    let unbalanced = Subdiagram {
        flows: vec![0.25, -0.25],
        prior: Some(0),
        connect: (2, 0),
        ..Subdiagram::default()
    };
    assert!(matches!(
        chart.add(unbalanced).err(),
        Some(SankeyError::Unbalanced { .. })
    ));
    let unknown = Subdiagram {
        prior: Some(5),
        ..Subdiagram::default()
    };
    assert_eq!(
        chart.add(unknown).err(),
        Some(SankeyError::UnknownPrior {
            prior: 5,
            diagrams: 2
        })
    );
}

#[test]
fn per_flow_arguments_must_match_the_flows() {
    let (_, mut chart) = sankey(SankeyOptions::default());
    let sub = Subdiagram {
        orientations: vec![FlowOrientation::Top; 3],
        ..Subdiagram::default()
    };
    assert_eq!(
        chart.add(sub).err(),
        Some(SankeyError::Count {
            argument: "orientations",
            len: 3,
            flows: 2
        })
    );
    let sub = Subdiagram {
        pathlengths: PathLengths::PerFlow(vec![0.1]),
        ..Subdiagram::default()
    };
    assert!(matches!(
        chart.add(sub).err(),
        Some(SankeyError::Count {
            argument: "pathlengths",
            ..
        })
    ));
}

#[test]
fn outlines_take_the_cycle_colour_or_the_classic_grey_blue() {
    let outline_style = |classic: &str| {
        crate::rc::context(|| {
            crate::rc::set("_internal.classic_mode", classic).unwrap();
            let (_, mut chart) = sankey(SankeyOptions::default());
            chart.add(Subdiagram::default()).unwrap();
            let patch = &chart.diagrams()[0].patch;
            (patch.colors().unwrap().face, patch.style().linewidth)
        })
    };
    let blue = Color::from("#1f77b4").resolve().unwrap();
    assert_eq!(outline_style("False"), (blue, None));
    let grey_blue = Color::from(CLASSIC_FACE).resolve().unwrap();
    assert_eq!(outline_style("True"), (grey_blue, Some(0.5)));
}
