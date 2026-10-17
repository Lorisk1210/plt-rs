//! The outline of one Sankey subdiagram: a trunk with input dips and
//! output arrows (matplotlib's `Sankey._arc`, `_add_input`, `_add_output`,
//! `_revert` and the path assembly of `Sankey.add`).

use plt_core::{PathCode, Point};

/// One path instruction, matplotlib's `(code, vertex)` pair.
pub(super) type Step = (PathCode, Point);

/// The direction of a flow's arrow before rotation (matplotlib's `RIGHT`,
/// `UP` and `DOWN`, in quarter turns).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Right,
    Up,
    Down,
}

impl Direction {
    /// The direction in quarter turns (`0`, `1` or `3`).
    pub(super) fn quarter_turns(self) -> f64 {
        match self {
            Self::Right => 0.0,
            Self::Up => 1.0,
            Self::Down => 3.0,
        }
    }

    /// `+1` upwards, `-1` downwards (horizontal arrows have no sign).
    fn sign(self) -> f64 {
        if self == Self::Up { 1.0 } else { -1.0 }
    }
}

/// A flow placed on the outline: its direction (`None` when it is too
/// small to draw), scaled size and arrow length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PlacedFlow {
    pub(super) direction: Option<Direction>,
    /// Whether the flow enters the diagram.
    pub(super) input: bool,
    pub(super) flow: f64,
    pub(super) length: f64,
}

/// The shape settings shared by every subdiagram.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Geometry {
    pub(super) gap: f64,
    pub(super) radius: f64,
    pub(super) shoulder: f64,
    pub(super) offset: f64,
    /// Slope of the arrow heads.
    pub(super) pitch: f64,
}

/// The assembled outline with a tip and label location per flow (the
/// origin for flows not drawn).
pub(super) struct Outline {
    pub(super) steps: Vec<Step>,
    pub(super) tips: Vec<Point>,
    pub(super) label_locations: Vec<Point>,
}

/// Control polygon of a cubic Bézier approximation of a quarter circle
/// (`Path.arc(0, 90)`, as tabulated by matplotlib).
#[allow(
    clippy::approx_constant,
    reason = "matplotlib tabulates the arc to nine digits, not 1/sqrt(2) exactly"
)]
const ARC_VERTICES: [(f64, f64); 7] = [
    (1.0, 0.0),
    (1.0, 2.651_147_73e-1),
    (8.945_712_35e-1, 5.196_423_27e-1),
    (7.071_067_81e-1, 7.071_067_81e-1),
    (5.196_423_27e-1, 8.945_712_35e-1),
    (2.651_147_73e-1, 1.0),
    (0.0, 1.0),
];

/// A quarter arc in `quadrant` (0–3), clockwise or not, of `radius` about
/// `center` (`Sankey._arc`).
fn arc(quadrant: u8, cw: bool, radius: f64, center: Point) -> Vec<Step> {
    let radius = if quadrant > 1 { -radius } else { radius };
    ARC_VERTICES
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let (vx, vy) = match (quadrant.is_multiple_of(2), cw) {
                (true, true) => (x, y),
                (true, false) => (y, x),
                (false, true) => (-y, x),
                (false, false) => (-x, y),
            };
            let code = if i == 0 {
                PathCode::LineTo
            } else {
                PathCode::Curve4
            };
            (
                code,
                Point::new(radius * vx + center.x, radius * vy + center.y),
            )
        })
        .collect()
}

fn line(x: f64, y: f64) -> Step {
    (PathCode::LineTo, Point::new(x, y))
}

/// The last vertex of a path under construction.
fn last(path: &[Step]) -> Point {
    path.last().expect("subpaths start with a vertex").1
}

impl Geometry {
    /// Appends an input to `path`; returns its dip and label location
    /// (`Sankey._add_input`).
    fn add_input(
        &self,
        path: &mut Vec<Step>,
        direction: Direction,
        flow: f64,
        length: f64,
    ) -> (Point, Point) {
        let Point { mut x, y } = last(path);
        let dipdepth = (flow / 2.0) * self.pitch;
        if direction == Direction::Right {
            x -= length;
            let dip = Point::new(x + dipdepth, y + flow / 2.0);
            path.extend([
                line(x, y),
                (PathCode::LineTo, dip),
                line(x, y + flow),
                line(x + self.gap, y + flow),
            ]);
            return (dip, Point::new(dip.x - self.offset, dip.y));
        }
        x -= self.gap;
        let sign = direction.sign();
        let dip = Point::new(x - flow / 2.0, y - sign * (length - dipdepth));
        let quadrant = if direction == Direction::Down { 2 } else { 1 };
        let center = Point::new(x + self.radius, y - sign * self.radius);
        if self.radius == 0.0 {
            path.push(line(x, y));
        } else {
            path.extend(arc(
                quadrant,
                direction == Direction::Up,
                self.radius,
                center,
            ));
        }
        path.extend([
            line(x, y - sign * length),
            (PathCode::LineTo, dip),
            line(x - flow, y - sign * length),
        ]);
        path.extend(arc(
            quadrant,
            direction == Direction::Down,
            flow + self.radius,
            center,
        ));
        path.push(line(x - flow, y + sign * flow));
        (dip, Point::new(dip.x, dip.y - sign * self.offset))
    }

    /// Appends an output (a negative `flow`) to `path`; returns its tip and
    /// label location (`Sankey._add_output`).
    fn add_output(
        &self,
        path: &mut Vec<Step>,
        direction: Direction,
        flow: f64,
        length: f64,
    ) -> (Point, Point) {
        let Point { mut x, y } = last(path);
        let tipheight = (self.shoulder - flow / 2.0) * self.pitch;
        if direction == Direction::Right {
            x += length;
            let tip = Point::new(x + tipheight, y + flow / 2.0);
            path.extend([
                line(x, y),
                line(x, y + self.shoulder),
                (PathCode::LineTo, tip),
                line(x, y - self.shoulder + flow),
                line(x, y + flow),
                line(x - self.gap, y + flow),
            ]);
            return (tip, Point::new(tip.x + self.offset, tip.y));
        }
        x += self.gap;
        let sign = direction.sign();
        let quadrant = if direction == Direction::Up { 3 } else { 0 };
        let tip = Point::new(x - flow / 2.0, y + sign * (length + tipheight));
        let center = Point::new(x - self.radius, y + sign * self.radius);
        if self.radius == 0.0 {
            path.push(line(x, y));
        } else {
            path.extend(arc(
                quadrant,
                direction == Direction::Up,
                self.radius,
                center,
            ));
        }
        path.extend([
            line(x, y + sign * length),
            line(x - self.shoulder, y + sign * length),
            (PathCode::LineTo, tip),
            line(x + self.shoulder - flow, y + sign * length),
            line(x - flow, y + sign * length),
        ]);
        path.extend(arc(
            quadrant,
            direction == Direction::Down,
            self.radius - flow,
            center,
        ));
        path.push(line(x - flow, y + sign * flow));
        (tip, Point::new(tip.x, tip.y + sign * self.offset))
    }

    /// The outline of a trunk of length `trunk` carrying `flows`, traced
    /// clockwise from the top (the path assembly of `Sankey.add`).
    pub(super) fn outline(&self, flows: &[PlacedFlow], trunk: f64) -> Outline {
        use Direction::{Down, Right, Up};
        let mut paths = Subpaths::trunk(self.gap, flows, trunk);
        let n = flows.len();
        let mut tips = vec![Point::ORIGIN; n];
        let mut labels = vec![Point::ORIGIN; n];
        let mut place = |i: usize, (tip, label): (Point, Point)| {
            tips[i] = tip;
            labels[i] = label;
        };
        // Top side, from the middle outwards.
        for (i, f) in flows.iter().enumerate() {
            match (f.direction, f.input) {
                (Some(Down), true) => {
                    place(
                        i,
                        self.add_input(&mut paths.upper_left, Down, f.flow, f.length),
                    );
                }
                (Some(Up), false) => {
                    place(
                        i,
                        self.add_output(&mut paths.upper_right, Up, f.flow, f.length),
                    );
                }
                _ => {}
            }
        }
        // Bottom side, from the middle outwards.
        for (i, f) in flows.iter().enumerate().rev() {
            match (f.direction, f.input) {
                (Some(Up), true) => {
                    place(
                        i,
                        self.add_input(&mut paths.lower_left, Up, f.flow, f.length),
                    );
                }
                (Some(Down), false) => {
                    place(
                        i,
                        self.add_output(&mut paths.lower_right, Down, f.flow, f.length),
                    );
                }
                _ => {}
            }
        }
        // Left-side inputs, from the bottom upwards.
        let mut has_left_input = false;
        for (i, f) in flows.iter().enumerate().rev() {
            if f.direction == Some(Right) && f.input {
                if !has_left_input {
                    // The lower path extends at least as far as the upper one.
                    let (lower, upper) = (last(&paths.lower_left), last(&paths.upper_left));
                    if lower.x > upper.x {
                        paths.lower_left.push(line(upper.x, lower.y));
                    }
                    has_left_input = true;
                }
                place(
                    i,
                    self.add_input(&mut paths.lower_left, Right, f.flow, f.length),
                );
            }
        }
        // Right-side outputs, from the top downwards.
        let mut has_right_output = false;
        for (i, f) in flows.iter().enumerate() {
            if f.direction == Some(Right) && !f.input {
                if !has_right_output {
                    // The upper path extends at least as far as the lower one.
                    let (upper, lower) = (last(&paths.upper_right), last(&paths.lower_right));
                    if upper.x < lower.x {
                        paths.upper_right.push(line(lower.x, upper.y));
                    }
                    has_right_output = true;
                }
                place(
                    i,
                    self.add_output(&mut paths.upper_right, Right, f.flow, f.length),
                );
            }
        }
        Outline {
            steps: paths.close(has_left_input, has_right_output),
            tips,
            label_locations: labels,
        }
    }
}

/// The four corners of an outline under construction, each growing from
/// the trunk outwards.
struct Subpaths {
    upper_right: Vec<Step>,
    lower_right: Vec<Step>,
    lower_left: Vec<Step>,
    upper_left: Vec<Step>,
}

impl Subpaths {
    /// The trunk: its top and bottom edges join the inputs to the
    /// outputs, curving where gain and loss differ.
    fn trunk(gap: f64, flows: &[PlacedFlow], trunk: f64) -> Self {
        let gain: f64 = flows.iter().map(|f| f.flow.max(0.0)).sum();
        let loss: f64 = flows.iter().map(|f| f.flow.min(0.0)).sum();
        let left = gap - trunk / 2.0;
        let right = trunk / 2.0 - gap;
        let p = Point::new;
        Self {
            upper_right: vec![
                (PathCode::MoveTo, p(left, gain / 2.0)),
                (PathCode::LineTo, p(left / 2.0, gain / 2.0)),
                (PathCode::Curve4, p(left / 8.0, gain / 2.0)),
                (PathCode::Curve4, p(right / 8.0, -loss / 2.0)),
                (PathCode::LineTo, p(right / 2.0, -loss / 2.0)),
                (PathCode::LineTo, p(right, -loss / 2.0)),
            ],
            lower_left: vec![
                (PathCode::LineTo, p(right, loss / 2.0)),
                (PathCode::LineTo, p(right / 2.0, loss / 2.0)),
                (PathCode::Curve4, p(right / 8.0, loss / 2.0)),
                (PathCode::Curve4, p(left / 8.0, -gain / 2.0)),
                (PathCode::LineTo, p(left / 2.0, -gain / 2.0)),
                (PathCode::LineTo, p(left, -gain / 2.0)),
            ],
            lower_right: vec![(PathCode::LineTo, p(right, loss / 2.0))],
            upper_left: vec![(PathCode::LineTo, p(left, gain / 2.0))],
        }
    }

    /// Trims the vertices left hanging without side flows and joins the
    /// corners clockwise from the top into a closed path.
    fn close(mut self, has_left_input: bool, has_right_output: bool) -> Vec<Step> {
        if !has_left_input {
            self.upper_left.pop();
            self.lower_left.pop();
        }
        if !has_right_output {
            self.lower_right.pop();
            self.upper_right.pop();
        }
        let start = self.upper_right[0].1;
        let mut steps = self.upper_right;
        steps.extend(revert(&self.lower_right));
        steps.extend(self.lower_left);
        steps.extend(revert(&self.upper_left));
        steps.push((PathCode::ClosePoly, start));
        steps
    }
}

/// `path` traced backwards: each vertex keeps the code that led away from
/// it, the first one a line (`Sankey._revert`).
fn revert(path: &[Step]) -> Vec<Step> {
    let mut next = PathCode::LineTo;
    path.iter()
        .rev()
        .map(|&(code, vertex)| {
            let step = (next, vertex);
            next = code;
            step
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEOMETRY: Geometry = Geometry {
        gap: 0.25,
        radius: 0.1,
        shoulder: 0.03,
        offset: 0.15,
        pitch: 0.839_099_631_177_280_1,
    };

    #[test]
    fn arcs_turn_and_scale_the_quarter_circle() {
        let steps = arc(2, false, 2.0, Point::new(1.0, 1.0));
        assert_eq!(steps[0], line(1.0, -1.0));
        assert_eq!(steps[6], (PathCode::Curve4, Point::new(-1.0, 1.0)));
        let steps = arc(1, true, 1.0, Point::ORIGIN);
        assert_eq!(steps[6].1, Point::new(-1.0, 0.0));
    }

    #[test]
    fn reverting_shifts_the_codes() {
        let path = [
            (PathCode::MoveTo, Point::new(0.0, 0.0)),
            (PathCode::Curve4, Point::new(1.0, 0.0)),
            (PathCode::Curve4, Point::new(2.0, 0.0)),
            (PathCode::LineTo, Point::new(3.0, 0.0)),
        ];
        let codes: Vec<PathCode> = revert(&path).iter().map(|s| s.0).collect();
        assert_eq!(
            codes,
            [
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::Curve4,
                PathCode::Curve4
            ]
        );
    }

    #[test]
    fn a_balanced_trunk_has_a_tip_per_flow() {
        let flows = [
            PlacedFlow {
                direction: Some(Direction::Right),
                input: true,
                flow: 1.0,
                length: 0.0,
            },
            PlacedFlow {
                direction: Some(Direction::Right),
                input: false,
                flow: -1.0,
                length: 0.0,
            },
        ];
        let outline = GEOMETRY.outline(&flows, 1.0);
        assert_eq!(outline.steps[0].0, PathCode::MoveTo);
        assert_eq!(outline.steps.last().map(|s| s.0), Some(PathCode::ClosePoly));
        // The output tip lies beyond the trunk by the arrow head; the input
        // dip points into it.
        assert!(outline.tips[1].x > 0.25);
        assert_eq!(
            outline.tips[0],
            Point::new(-0.25 + 0.5 * GEOMETRY.pitch, 0.0)
        );
        assert_eq!(outline.label_locations[1].y, outline.tips[1].y);
    }
}
