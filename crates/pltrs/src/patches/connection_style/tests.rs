use pltrs_core::{Path, PathCode, Point};

use super::*;
use crate::testing::close_points;

fn style(text: &str) -> ConnectionStyle {
    text.parse().expect("valid connection style")
}

const A: Point = Point::new(0.0, 0.0);
const B: Point = Point::new(100.0, 100.0);

#[test]
fn descriptions_parse_back() {
    for text in [
        "arc3,rad=0.3",
        "angle3,angleA=-90,angleB=180",
        "angle,angleA=0,angleB=90,rad=5",
        "arc,angleA=0,angleB=180,armA=30,armB=0,rad=10",
        "arc",
        "bar,armA=1,armB=2,fraction=-0.3,angle=45",
        "bar",
    ] {
        let parsed = style(text);
        assert_eq!(style(&parsed.to_string()), parsed, "{text}");
    }
    assert_eq!(style("arc3").to_string(), "arc3,rad=0");
    assert_eq!(style("bar").to_string(), "bar,armA=0,armB=0,fraction=0.3");
}

#[test]
fn parses_names_and_defaults() {
    assert_eq!(style("arc3"), ConnectionStyle::Arc3 { rad: 0.0 });
    assert_eq!(
        style("angle3"),
        ConnectionStyle::Angle3 {
            angle_a: 90.0,
            angle_b: 0.0
        }
    );
    assert_eq!(
        style("bar,fraction=-0.3"),
        ConnectionStyle::Bar {
            arm_a: 0.0,
            arm_b: 0.0,
            fraction: -0.3,
            angle: None
        }
    );
    assert!("arc3,armA=3".parse::<ConnectionStyle>().is_err());
    assert!("spline".parse::<ConnectionStyle>().is_err());
}

#[test]
fn arc3_bends_to_the_right_of_travel() {
    let path = style("arc3,rad=0.3").connect(A, B).unwrap();
    assert_eq!(path.vertices()[1], Point::new(80.0, 20.0));
    assert_eq!(path.code_at(2), PathCode::Curve3);
}

#[test]
fn angles_meet_at_a_corner() {
    let path = style("angle3,angleA=90,angleB=0").connect(A, B).unwrap();
    assert!(close_points(path.vertices()[1], Point::new(0.0, 100.0)));
    let sharp = style("angle,angleA=-90,angleB=180,rad=0")
        .connect(A, B)
        .unwrap();
    assert_eq!(sharp.len(), 3);
    assert!(close_points(sharp.vertices()[1], Point::new(0.0, 100.0)));
    let round = style("angle,angleA=-90,angleB=180,rad=5")
        .connect(A, B)
        .unwrap();
    assert_eq!(round.len(), 5);
    assert!(close_points(round.vertices()[1], Point::new(0.0, 95.0)));
    assert!(close_points(round.vertices()[3], Point::new(5.0, 100.0)));
    assert_eq!(
        style("angle3,angleA=0,angleB=180").connect(A, B),
        Err(PatchError::ParallelAngles)
    );
}

#[test]
fn arcs_leave_along_their_arms() {
    let path = style("arc,angleA=-90,angleB=0,armA=30,armB=30,rad=0")
        .connect(A, B)
        .unwrap();
    let v = path.vertices();
    assert_eq!(path.len(), 8);
    assert!(close_points(v[2], Point::new(0.0, -30.0)));
    assert!(close_points(v[5], Point::new(130.0, 100.0)));
    assert_eq!(*v.last().unwrap(), B);
    let plain = style("arc").connect(A, B).unwrap();
    assert_eq!(plain.len(), 2);
}

#[test]
fn bars_stand_off_the_line() {
    let path = style("bar,fraction=0.1").connect(A, B).unwrap();
    let f = 0.1 * 100.0 * 2f64.sqrt();
    let s = f / 2f64.sqrt();
    assert!(close_points(path.vertices()[1], Point::new(s, -s)));
    assert!(close_points(
        path.vertices()[2],
        Point::new(100.0 + s, 100.0 - s)
    ));
}

#[test]
fn routes_are_shrunk_at_both_ends() {
    let path = style("arc3")
        .route((A, Point::new(100.0, 0.0)), (10.0, 20.0), None, None)
        .unwrap();
    let v = path.vertices();
    assert!((v[0].x - 10.0).abs() < 0.02);
    assert!((v[v.len() - 1].x - 80.0).abs() < 0.02);
}

#[test]
fn routes_of_the_program_shrink_themselves() {
    let route = ProgramRoute::new(std::rc::Rc::new(|(a, b): (Point, Point), (sa, _)| {
        Path::polyline(vec![Point::new(a.x + sa, a.y), b])
    }));
    let path = ConnectionStyle::ProgramRoute(route)
        .route((A, Point::new(100.0, 0.0)), (10.0, 20.0), None, None)
        .unwrap();
    assert_eq!(
        path.vertices(),
        [Point::new(10.0, 0.0), Point::new(100.0, 0.0)]
    );
}

#[test]
fn routes_start_at_the_edge_of_their_patch() {
    let square = ClipRegion {
        path: Path::polygon([
            Point::new(-10.0, -10.0),
            Point::new(10.0, -10.0),
            Point::new(10.0, 10.0),
            Point::new(-10.0, 10.0),
        ]),
        radius: 0.0,
    };
    let path = style("arc3")
        .route((A, Point::new(100.0, 0.0)), (0.0, 0.0), Some(&square), None)
        .unwrap();
    assert!((path.vertices()[0].x - 10.0).abs() < 0.02);
    assert!(square.contains(Point::new(5.0, 5.0)));
    assert!(!square.contains(Point::new(15.0, 5.0)));
}

#[test]
fn coincident_points_have_no_direction() {
    for name in [
        "bar",
        "bar,angle=45",
        "arc,angleA=0,angleB=0,armA=30,armB=30",
    ] {
        assert_eq!(
            style(name).connect(A, A),
            Err(PatchError::CoincidentPoints),
            "{name}"
        );
    }
    assert!(
        style("arc3").connect(A, A).is_ok(),
        "arc3 divides by nothing"
    );
}
