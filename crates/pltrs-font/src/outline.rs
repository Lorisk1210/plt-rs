//! Glyph outlines: the [`OutlineSink`] trait and a FreeType-faithful contour
//! decomposer.
//!
//! Glyph loaders (TrueType `glyf`, CFF) fill an [`Outline`] with tagged
//! points; [`Outline::decompose`] then turns each contour into
//! `move_to`/`line_to`/`quad_to`/`curve_to`/`close` calls exactly like
//! `FT_Outline_Decompose`, including implied on-curve points between
//! consecutive quadratic control points and the explicit closing line back to
//! the contour start.

/// Receives a glyph outline, one contour at a time.
///
/// Each contour starts with [`move_to`](Self::move_to) and ends with
/// [`close`](Self::close). Coordinates are in font units with y pointing up,
/// unless the producing API documents otherwise.
pub trait OutlineSink {
    /// Starts a new contour at `(x, y)`.
    fn move_to(&mut self, x: f32, y: f32);
    /// Adds a straight segment to `(x, y)`.
    fn line_to(&mut self, x: f32, y: f32);
    /// Adds a quadratic Bézier segment with control point `(x1, y1)`.
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32);
    /// Adds a cubic Bézier segment with control points `(x1, y1)`, `(x2, y2)`.
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);
    /// Closes the current contour.
    fn close(&mut self);
}

/// Role of an outline point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum PointKind {
    OnCurve,
    Quadratic,
    Cubic,
}

/// One tagged outline point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct OutlinePoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) kind: PointKind,
}

/// How implied midpoints are computed during decomposition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Midpoint {
    /// Exact halves, for outlines in font units.
    Exact,
    /// C integer division `(a + b) / 2`, for FreeType 26.6 outlines.
    Truncated,
    /// `(a + b) / 2` rounded down: how FreeType's rasterizers see the
    /// midpoints of a 26.6 outline, which they decompose after moving it
    /// into their bitmap, where every coordinate is non-negative.
    Floored,
}

impl Midpoint {
    fn of(self, a: f64, b: f64) -> f64 {
        match self {
            Self::Exact => f64::midpoint(a, b),
            Self::Truncated => f64::midpoint(a, b).trunc(),
            Self::Floored => f64::midpoint(a, b).floor(),
        }
    }
}

/// A list of contours made of tagged points.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outline {
    pub(crate) points: Vec<OutlinePoint>,
    /// Exclusive end index of each contour in `points`.
    pub(crate) contour_ends: Vec<usize>,
}

impl Outline {
    pub(crate) fn push(&mut self, x: f64, y: f64, kind: PointKind) {
        self.points.push(OutlinePoint { x, y, kind });
    }

    /// Ends the current contour; empty contours are dropped.
    pub(crate) fn end_contour(&mut self) {
        let start = self.contour_ends.last().copied().unwrap_or(0);
        if self.points.len() > start {
            self.contour_ends.push(self.points.len());
        }
    }

    /// Moves every point by `(dx, dy)`.
    pub(crate) fn translate(&mut self, dx: f64, dy: f64) {
        for point in &mut self.points {
            point.x += dx;
            point.y += dy;
        }
    }

    /// Control box over all points (on- and off-curve):
    /// `(x_min, y_min, x_max, y_max)`.
    pub(crate) fn control_box(&self) -> Option<(f64, f64, f64, f64)> {
        let first = self.points.first()?;
        let init = (first.x, first.y, first.x, first.y);
        Some(self.points.iter().fold(init, |(x0, y0, x1, y1), p| {
            (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y))
        }))
    }

    /// Emits every contour into `sink` following `FT_Outline_Decompose`.
    /// Contours that FreeType would reject (starting with a cubic control
    /// point, or with dangling cubic controls) are skipped.
    pub(crate) fn decompose(&self, sink: &mut dyn OutlineSink, midpoint: Midpoint) {
        let mut start = 0;
        for &end in &self.contour_ends {
            if let Some(contour) = self.points.get(start..end) {
                decompose_contour(contour, sink, midpoint);
            }
            start = end;
        }
    }
}

fn emit_move(sink: &mut dyn OutlineSink, p: (f64, f64)) {
    sink.move_to(p.0 as f32, p.1 as f32);
}

fn emit_line(sink: &mut dyn OutlineSink, p: (f64, f64)) {
    sink.line_to(p.0 as f32, p.1 as f32);
}

fn emit_quad(sink: &mut dyn OutlineSink, c: (f64, f64), p: (f64, f64)) {
    sink.quad_to(c.0 as f32, c.1 as f32, p.0 as f32, p.1 as f32);
}

/// Validates a contour the way FreeType's decomposer would accept it.
fn is_valid_contour(contour: &[OutlinePoint]) -> bool {
    let Some(first) = contour.first() else {
        return false;
    };
    if first.kind == PointKind::Cubic {
        return false;
    }
    // Cubic control points must come in pairs followed by a point (or the
    // contour start).
    let mut run = 0;
    for point in contour {
        if point.kind == PointKind::Cubic {
            run += 1;
            if run > 2 {
                return false;
            }
        } else {
            if run == 1 {
                return false;
            }
            run = 0;
        }
    }
    run != 1
}

fn decompose_contour(contour: &[OutlinePoint], sink: &mut dyn OutlineSink, midpoint: Midpoint) {
    if !is_valid_contour(contour) {
        return;
    }
    let xy = |p: &OutlinePoint| (p.x, p.y);
    let first = &contour[0];
    let last = &contour[contour.len() - 1];
    let mut rest = &contour[1..];
    let start = if first.kind == PointKind::Quadratic {
        if last.kind == PointKind::OnCurve {
            // Start at the last point; it is consumed as the start.
            rest = &contour[..contour.len() - 1];
            xy(last)
        } else {
            rest = contour;
            (midpoint.of(first.x, last.x), midpoint.of(first.y, last.y))
        }
    } else {
        xy(first)
    };
    emit_move(sink, start);

    let mut index = 0;
    while index < rest.len() {
        let point = &rest[index];
        match point.kind {
            PointKind::OnCurve => {
                emit_line(sink, xy(point));
                index += 1;
            }
            PointKind::Quadratic => {
                let mut control = xy(point);
                index += 1;
                loop {
                    let Some(next) = rest.get(index) else {
                        emit_quad(sink, control, start);
                        sink.close();
                        return;
                    };
                    index += 1;
                    if next.kind == PointKind::OnCurve {
                        emit_quad(sink, control, xy(next));
                        break;
                    }
                    let middle = (
                        midpoint.of(control.0, next.x),
                        midpoint.of(control.1, next.y),
                    );
                    emit_quad(sink, control, middle);
                    control = xy(next);
                }
            }
            PointKind::Cubic => {
                let c1 = xy(point);
                let c2 = rest.get(index + 1).map_or(start, xy);
                let end = rest.get(index + 2).map_or(start, xy);
                sink.curve_to(
                    c1.0 as f32,
                    c1.1 as f32,
                    c2.0 as f32,
                    c2.1 as f32,
                    end.0 as f32,
                    end.1 as f32,
                );
                if index + 2 >= rest.len() {
                    sink.close();
                    return;
                }
                index += 3;
            }
        }
    }
    emit_line(sink, start);
    sink.close();
}

/// An [`OutlineSink`] that records every call; handy for tests and for
/// callers that want a simple owned representation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlineRecorder {
    /// Recorded commands in call order.
    pub commands: Vec<PathCommand>,
}

/// One recorded outline command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    /// Contour start.
    MoveTo(f32, f32),
    /// Straight segment.
    LineTo(f32, f32),
    /// Quadratic segment: control point then end point.
    QuadTo(f32, f32, f32, f32),
    /// Cubic segment: two control points then end point.
    CurveTo(f32, f32, f32, f32, f32, f32),
    /// Contour end.
    Close,
}

impl OutlineSink for OutlineRecorder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::MoveTo(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(PathCommand::LineTo(x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.commands.push(PathCommand::QuadTo(x1, y1, x, y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.commands
            .push(PathCommand::CurveTo(x1, y1, x2, y2, x, y));
    }
    fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::PathCommand::{Close, CurveTo, LineTo, MoveTo, QuadTo};
    use super::*;

    fn outline(points: &[(f64, f64, PointKind)]) -> Outline {
        let mut outline = Outline::default();
        for &(x, y, kind) in points {
            outline.push(x, y, kind);
        }
        outline.end_contour();
        outline
    }

    fn record(outline: &Outline, midpoint: Midpoint) -> Vec<PathCommand> {
        let mut recorder = OutlineRecorder::default();
        outline.decompose(&mut recorder, midpoint);
        recorder.commands
    }

    use PointKind::{Cubic, OnCurve as On, Quadratic as Off};

    #[test]
    fn polygon_closes_with_explicit_line() {
        let o = outline(&[(0.0, 0.0, On), (10.0, 0.0, On), (10.0, 10.0, On)]);
        assert_eq!(
            record(&o, Midpoint::Exact),
            vec![
                MoveTo(0.0, 0.0),
                LineTo(10.0, 0.0),
                LineTo(10.0, 10.0),
                LineTo(0.0, 0.0),
                Close
            ]
        );
        assert_eq!(o.control_box(), Some((0.0, 0.0, 10.0, 10.0)));
    }

    #[test]
    fn implied_on_curve_points() {
        let o = outline(&[
            (0.0, 0.0, On),
            (1.0, 2.0, Off),
            (3.0, 2.0, Off),
            (4.0, 0.0, On),
        ]);
        assert_eq!(
            record(&o, Midpoint::Exact),
            vec![
                MoveTo(0.0, 0.0),
                QuadTo(1.0, 2.0, 2.0, 2.0),
                QuadTo(3.0, 2.0, 4.0, 0.0),
                LineTo(0.0, 0.0),
                Close
            ]
        );
    }

    #[test]
    fn contour_starting_off_curve() {
        // Last point on-curve: start there.
        let o = outline(&[(1.0, 1.0, Off), (2.0, 0.0, On), (0.0, 0.0, On)]);
        assert_eq!(
            record(&o, Midpoint::Exact),
            vec![
                MoveTo(0.0, 0.0),
                QuadTo(1.0, 1.0, 2.0, 0.0),
                LineTo(0.0, 0.0),
                Close
            ]
        );
        // All off-curve: start at the midpoint of first and last, truncating
        // in 26.6 mode.
        let o = outline(&[
            (0.0, 0.0, Off),
            (3.0, 0.0, Off),
            (3.0, 3.0, Off),
            (0.0, 3.0, Off),
        ]);
        let cmds = record(&o, Midpoint::Truncated);
        assert_eq!(cmds[0], MoveTo(0.0, 1.0));
        assert_eq!(cmds.last(), Some(&Close));
        assert_eq!(cmds[cmds.len() - 2], QuadTo(0.0, 3.0, 0.0, 1.0));
        assert_eq!(cmds.len(), 6);
    }

    #[test]
    fn cubic_segments() {
        let o = outline(&[
            (0.0, 0.0, On),
            (1.0, 1.0, Cubic),
            (2.0, 1.0, Cubic),
            (3.0, 0.0, On),
        ]);
        assert_eq!(
            record(&o, Midpoint::Exact),
            vec![
                MoveTo(0.0, 0.0),
                CurveTo(1.0, 1.0, 2.0, 1.0, 3.0, 0.0),
                LineTo(0.0, 0.0),
                Close
            ]
        );
        let closing = outline(&[(0.0, 0.0, On), (1.0, 1.0, Cubic), (2.0, 1.0, Cubic)]);
        assert_eq!(
            record(&closing, Midpoint::Exact),
            vec![
                MoveTo(0.0, 0.0),
                CurveTo(1.0, 1.0, 2.0, 1.0, 0.0, 0.0),
                Close
            ]
        );
    }

    #[test]
    fn invalid_contours_are_skipped() {
        let o = outline(&[(0.0, 0.0, Cubic), (1.0, 1.0, On)]);
        assert_eq!(record(&o, Midpoint::Exact), Vec::<PathCommand>::new());
        let o = outline(&[(0.0, 0.0, On), (1.0, 1.0, Cubic), (1.0, 1.0, On)]);
        assert_eq!(record(&o, Midpoint::Exact), Vec::<PathCommand>::new());
    }

    #[test]
    fn single_point_and_empty_contours() {
        let mut o = outline(&[(5.0, 5.0, On)]);
        o.end_contour();
        assert_eq!(o.contour_ends.len(), 1);
        assert_eq!(
            record(&o, Midpoint::Exact),
            vec![MoveTo(5.0, 5.0), LineTo(5.0, 5.0), Close]
        );
        assert!(Outline::default().control_box().is_none());
        o.translate(1.0, -1.0);
        assert_eq!(o.points[0].x, 6.0);
    }
}
