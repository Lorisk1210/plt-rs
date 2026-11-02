use pltrs_core::{Path, PathCode, Point};

use super::*;
use crate::testing::horizontal_segment;

fn style(text: &str) -> ArrowStyle {
    text.parse().expect("valid arrow style")
}

fn arc() -> Path {
    Path::new(
        vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 30.0),
            Point::new(100.0, 0.0),
        ],
        vec![PathCode::MoveTo, PathCode::Curve3, PathCode::Curve3],
    )
    .unwrap()
}

#[test]
fn parses_every_name() {
    for name in [
        "-", "->", "<-", "<->", "-|>", "<|-", "<|-|>", "]-", "-[", "]-[", "|-|", "]->", "<-[",
        "simple", "fancy", "wedge",
    ] {
        assert!(name.parse::<ArrowStyle>().is_ok(), "{name}");
    }
    assert!("=>".parse::<ArrowStyle>().is_err());
    assert!("-[,head_length=1".parse::<ArrowStyle>().is_err());
    assert!("|-|,lengthA=1".parse::<ArrowStyle>().is_err());
    let ArrowStyle::Curve(bars) = style("|-|,widthA=0.3") else {
        panic!("a curve style");
    };
    assert_eq!((bars.bracket_a.width, bars.bracket_a.length), (0.3, 0.0));
}

#[test]
fn descriptions_parse_back() {
    for text in [
        "-",
        "->,head_length=0.6,widthA=2,scaleB=3",
        "<|-|>",
        "]-,widthA=1.5,lengthA=0.4,angleA=30",
        "-[,widthB=2",
        "]-[,lengthA=0.1",
        "|-|,widthA=0.3,angleB=10",
        "]->",
        "<-[,angleB=5",
        "simple,tail_width=0.5",
        "fancy",
        "wedge,shrink_factor=0.2",
    ] {
        let parsed = style(text);
        let described = parsed.to_string();
        assert_eq!(style(&described), parsed, "{text} described as {described}");
    }
    assert_eq!(
        style("|-|").to_string(),
        "|-|,widthA=1,angleA=0,widthB=1,angleB=0"
    );
    assert_eq!(style("]-").to_string(), "]-,widthA=1,angleA=0,lengthA=0.2");
    let unnamed = ArrowStyle::curve(CurveEnd::Bracket, CurveEnd::FilledHead);
    assert!(unnamed.to_string().parse::<ArrowStyle>().is_err());
}

#[test]
fn open_heads_are_stroked_and_filled_heads_filled() {
    let open = style("->")
        .paths(&horizontal_segment(), 10.0, 1.0, 1.0)
        .unwrap();
    assert_eq!(open.len(), 2);
    assert_eq!((open[0].1, open[1].1), (false, false));
    // The head's tip is pulled back by half the line width over sin(t).
    let tip = open[1].0.vertices()[1];
    let sin = 2.0 / 4.0_f64.hypot(2.0);
    assert!((tip.x - (100.0 - 0.5 / sin)).abs() < 1e-9);
    assert_eq!(open[0].0.vertices()[1], tip);

    let filled = style("<|-|>")
        .paths(&horizontal_segment(), 10.0, 1.0, 1.0)
        .unwrap();
    assert_eq!(filled.len(), 3);
    assert!(filled[1].1 && filled[2].1);
    assert_eq!(filled[1].0.code_at(3), PathCode::ClosePoly);
}

#[test]
fn brackets_stand_across_the_ends() {
    let paths = style("]-[")
        .paths(&horizontal_segment(), 10.0, 1.0, 1.0)
        .unwrap();
    assert_eq!(paths.len(), 3);
    let end = paths[2].0.vertices();
    // Width 1 × 10 on either side of the end, arms 0.2 × 10 long outwards.
    assert_eq!(end[1], Point::new(100.0, -10.0));
    assert_eq!(end[0], Point::new(102.0, -10.0));
    let plain = style("-")
        .paths(&horizontal_segment(), 10.0, 1.0, 1.0)
        .unwrap();
    assert_eq!(plain, vec![(horizontal_segment(), false)]);
}

#[test]
fn quadratic_styles_need_a_quadratic_curve() {
    assert_eq!(
        style("simple").paths(&horizontal_segment(), 10.0, 1.0, 1.0),
        Err(PatchError::NotQuadratic)
    );
    for name in ["simple", "fancy", "wedge"] {
        let paths = style(name).paths(&arc(), 10.0, 1.0, 1.0).unwrap();
        assert_eq!(paths.len(), 1, "{name}");
        assert!(paths[0].1);
        let outline = &paths[0].0;
        assert_eq!(outline.code_at(outline.len() - 1), PathCode::ClosePoly);
    }
}

#[test]
fn short_curves_get_a_straight_head() {
    let short = Path::new(
        vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 0.0),
        ],
        vec![PathCode::MoveTo, PathCode::Curve3, PathCode::Curve3],
    )
    .unwrap();
    let simple = style("simple").paths(&short, 20.0, 1.0, 1.0).unwrap();
    assert_eq!(simple[0].0.len(), 6, "head only");
}

#[test]
fn aspect_restores_multi_piece_arrows() {
    let squeezed = style("->").paths(&arc(), 10.0, 1.0, 2.0).unwrap();
    assert_eq!(squeezed[0].0.vertices()[1], Point::new(50.0, 30.0));
}
