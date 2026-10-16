//! Type 2 charstring interpreter producing cubic outlines.

use crate::cff::index::{CFF, Index};
use crate::error::{FontError, Result};
use crate::outline::{Outline, PointKind};
use crate::reader::Reader;

const MAX_STACK: usize = 48;
const MAX_CALL_DEPTH: u8 = 10;

/// Subroutine context of one glyph.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Subrs<'a> {
    pub(crate) global: Index<'a>,
    pub(crate) local: Index<'a>,
}

fn bias(count: usize) -> i64 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// An accented character request from `endchar` (`seac`): base and accent
/// character codes in Standard Encoding plus the accent offset.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Seac {
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) base: u8,
    pub(crate) accent: u8,
}

/// Builds contours the way FreeType's PostScript builder does: a move only
/// opens a contour once something is drawn, closing drops a final point
/// equal to the first, and single-point contours vanish.
struct Builder<'o> {
    outline: &'o mut Outline,
    x: f64,
    y: f64,
    open: bool,
    contour_start: usize,
}

impl Builder<'_> {
    fn close(&mut self) {
        if !self.open {
            return;
        }
        self.open = false;
        let points = &mut self.outline.points;
        if points.len() > self.contour_start + 1 {
            let first = points[self.contour_start];
            let last = points[points.len() - 1];
            let same =
                first.x.to_bits() == last.x.to_bits() && first.y.to_bits() == last.y.to_bits();
            if same && last.kind == PointKind::OnCurve {
                points.pop();
            }
        }
        if points.len() == self.contour_start + 1 {
            points.pop();
        } else {
            self.outline.end_contour();
        }
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        self.close();
        self.x += dx;
        self.y += dy;
    }

    fn begin(&mut self) {
        if !self.open {
            self.open = true;
            self.contour_start = self.outline.points.len();
            self.outline.push(self.x, self.y, PointKind::OnCurve);
        }
    }

    fn line_by(&mut self, dx: f64, dy: f64) {
        self.begin();
        self.x += dx;
        self.y += dy;
        self.outline.push(self.x, self.y, PointKind::OnCurve);
    }

    fn curve_by(&mut self, d: [f64; 6]) {
        self.begin();
        let (x1, y1) = (self.x + d[0], self.y + d[1]);
        let (x2, y2) = (x1 + d[2], y1 + d[3]);
        (self.x, self.y) = (x2 + d[4], y2 + d[5]);
        self.outline.push(x1, y1, PointKind::Cubic);
        self.outline.push(x2, y2, PointKind::Cubic);
        self.outline.push(self.x, self.y, PointKind::OnCurve);
    }
}

/// Interpreter state for one glyph.
struct Interpreter<'a, 'o> {
    subrs: Subrs<'a>,
    stack: Vec<f64>,
    builder: Builder<'o>,
    stems: usize,
    width_parsed: bool,
    seac: Option<Seac>,
}

/// Runs `charstring`, appending its contours to `outline`. Returns the
/// `seac` request when the glyph is an accented composite.
pub(crate) fn interpret(
    charstring: &[u8],
    subrs: Subrs<'_>,
    outline: &mut Outline,
) -> Result<Option<Seac>> {
    let builder = Builder {
        outline,
        x: 0.0,
        y: 0.0,
        open: false,
        contour_start: 0,
    };
    let mut interpreter = Interpreter {
        subrs,
        stack: Vec::with_capacity(MAX_STACK),
        builder,
        stems: 0,
        width_parsed: false,
        seac: None,
    };
    interpreter.run(charstring, 0)?;
    interpreter.builder.close();
    Ok(interpreter.seac)
}

/// Outcome of executing a charstring fragment.
#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Return,
    End,
}

impl Interpreter<'_, '_> {
    fn push(&mut self, value: f64) -> Result<()> {
        if self.stack.len() >= MAX_STACK {
            return Err(FontError::malformed(CFF, "charstring stack overflow"));
        }
        self.stack.push(value);
        Ok(())
    }

    /// Drops the optional leading width argument of the first
    /// stack-clearing operator when the stack has an odd extra value.
    fn take_width(&mut self, odd_means_width: bool) {
        if !self.width_parsed {
            self.width_parsed = true;
            if odd_means_width && !self.stack.is_empty() {
                self.stack.remove(0);
            }
        }
    }

    fn run(&mut self, code: &[u8], depth: u8) -> Result<Flow> {
        if depth > MAX_CALL_DEPTH {
            return Err(FontError::malformed(CFF, "subroutine nesting too deep"));
        }
        let mut r = Reader::new(code, CFF);
        while r.remaining() > 0 {
            let b0 = r.u8()?;
            let flow = match b0 {
                28 => {
                    self.push(f64::from(r.i16()?))?;
                    Flow::Continue
                }
                32..=246 => {
                    self.push(f64::from(i32::from(b0) - 139))?;
                    Flow::Continue
                }
                247..=250 => {
                    self.push(f64::from(
                        (i32::from(b0) - 247) * 256 + i32::from(r.u8()?) + 108,
                    ))?;
                    Flow::Continue
                }
                251..=254 => {
                    self.push(f64::from(
                        -(i32::from(b0) - 251) * 256 - i32::from(r.u8()?) - 108,
                    ))?;
                    Flow::Continue
                }
                255 => {
                    self.push(f64::from(r.i32()?) / 65536.0)?;
                    Flow::Continue
                }
                10 | 29 => self.call(b0 == 29, depth)?,
                11 => Flow::Return,
                19 | 20 => {
                    // hintmask/cntrmask: pending arguments are implicit vstems.
                    self.stems_op();
                    r.skip(self.stems.div_ceil(8))?;
                    Flow::Continue
                }
                _ => self.operator(b0, &mut r)?,
            };
            if flow != Flow::Continue {
                return Ok(flow);
            }
        }
        Ok(Flow::Continue)
    }

    fn call(&mut self, global: bool, depth: u8) -> Result<Flow> {
        let index = self
            .stack
            .pop()
            .ok_or(FontError::malformed(CFF, "callsubr on empty stack"))?;
        let subrs = if global {
            self.subrs.global
        } else {
            self.subrs.local
        };
        let number = index as i64 + bias(subrs.len());
        let code = usize::try_from(number)
            .ok()
            .and_then(|n| subrs.get(n))
            .ok_or(FontError::malformed(CFF, "subroutine index out of range"))?;
        match self.run(code, depth + 1)? {
            Flow::End => Ok(Flow::End),
            Flow::Continue | Flow::Return => Ok(Flow::Continue),
        }
    }

    fn stems_op(&mut self) {
        let odd = self.stack.len() % 2 == 1;
        self.take_width(odd);
        self.stems += self.stack.len() / 2;
        self.stack.clear();
    }

    fn operator(&mut self, op: u8, r: &mut Reader<'_>) -> Result<Flow> {
        let args = std::mem::take(&mut self.stack);
        match op {
            1 | 3 | 18 | 23 => {
                self.stack = args;
                self.stems_op();
            }
            21 | 22 | 4 => self.moveto(op, args),
            5 => self.lines(&args),
            6 | 7 => self.alternating_lines(&args, op == 6),
            8 => self.rrcurveto(&args),
            24 => {
                let split = args.len().saturating_sub(2);
                self.rrcurveto(&args[..split]);
                if let [dx, dy] = args[split..] {
                    self.builder.line_by(dx, dy);
                }
            }
            25 => {
                let split = args.len().saturating_sub(6);
                self.lines(&args[..split]);
                self.rrcurveto(&args[split..]);
            }
            26 => self.vv_hh_curveto(&args, false),
            27 => self.vv_hh_curveto(&args, true),
            30 => self.alternating_curves(&args, false),
            31 => self.alternating_curves(&args, true),
            14 => return self.endchar(args),
            12 => match r.u8()? {
                0 => {}
                escaped => self.flex(escaped, &args)?,
            },
            _ => return Err(FontError::malformed(CFF, "unsupported charstring operator")),
        }
        Ok(Flow::Continue)
    }

    fn moveto(&mut self, op: u8, mut args: Vec<f64>) {
        let expected = if op == 21 { 2 } else { 1 };
        if !self.width_parsed {
            self.width_parsed = true;
            if args.len() > expected {
                args.remove(0);
            }
        }
        match (op, args.as_slice()) {
            (21, [dx, dy, ..]) => self.builder.move_by(*dx, *dy),
            (22, [dx, ..]) => self.builder.move_by(*dx, 0.0),
            (4, [dy, ..]) => self.builder.move_by(0.0, *dy),
            _ => self.builder.move_by(0.0, 0.0),
        }
    }

    fn endchar(&mut self, mut args: Vec<f64>) -> Result<Flow> {
        if !self.width_parsed {
            self.width_parsed = true;
            if args.len() == 1 || args.len() == 5 {
                args.remove(0);
            }
        }
        if let [dx, dy, base, accent] = args[..] {
            let code =
                |v: f64| u8::try_from(v as i64).map_err(|_| FontError::malformed(CFF, "seac code"));
            self.seac = Some(Seac {
                dx,
                dy,
                base: code(base)?,
                accent: code(accent)?,
            });
        }
        Ok(Flow::End)
    }

    fn alternating_lines(&mut self, args: &[f64], mut horizontal: bool) {
        for &d in args {
            if horizontal {
                self.builder.line_by(d, 0.0);
            } else {
                self.builder.line_by(0.0, d);
            }
            horizontal = !horizontal;
        }
    }

    fn rrcurveto(&mut self, args: &[f64]) {
        for &d in args.as_chunks::<6>().0 {
            self.builder.curve_by(d);
        }
    }

    /// `rlineto` pairs.
    fn lines(&mut self, args: &[f64]) {
        for &[dx, dy] in args.as_chunks::<2>().0 {
            self.builder.line_by(dx, dy);
        }
    }

    /// `vvcurveto` / `hhcurveto`: optional leading orthogonal delta, then
    /// groups of four.
    fn vv_hh_curveto(&mut self, args: &[f64], horizontal: bool) {
        let (mut extra, rest) = if args.len() % 4 == 1 {
            (args[0], &args[1..])
        } else {
            (0.0, args)
        };
        for d in rest.as_chunks::<4>().0 {
            if horizontal {
                self.builder.curve_by([d[0], extra, d[1], d[2], d[3], 0.0]);
            } else {
                self.builder.curve_by([extra, d[0], d[1], d[2], 0.0, d[3]]);
            }
            extra = 0.0;
        }
    }

    /// `hvcurveto` / `vhcurveto`: curves alternating between horizontal and
    /// vertical tangents; a final extra value bends the last curve's end.
    fn alternating_curves(&mut self, args: &[f64], mut horizontal: bool) {
        let groups = args.len() / 4;
        let extra = if args.len() % 4 == 1 {
            args[args.len() - 1]
        } else {
            0.0
        };
        for (index, d) in args.as_chunks::<4>().0.iter().enumerate() {
            let last = if index + 1 == groups { extra } else { 0.0 };
            if horizontal {
                self.builder.curve_by([d[0], 0.0, d[1], d[2], last, d[3]]);
            } else {
                self.builder.curve_by([0.0, d[0], d[1], d[2], d[3], last]);
            }
            horizontal = !horizontal;
        }
    }

    fn flex(&mut self, op: u8, a: &[f64]) -> Result<()> {
        match (op, a) {
            (35, [d @ .., _fd]) if d.len() == 12 => {
                self.builder.curve_by([d[0], d[1], d[2], d[3], d[4], d[5]]);
                self.builder
                    .curve_by([d[6], d[7], d[8], d[9], d[10], d[11]]);
            }
            (34, &[dx1, dx2, dy2, dx3, dx4, dx5, dx6]) => {
                self.builder.curve_by([dx1, 0.0, dx2, dy2, dx3, 0.0]);
                self.builder.curve_by([dx4, 0.0, dx5, -dy2, dx6, 0.0]);
            }
            (36, &[dx1, dy1, dx2, dy2, dx3, dx4, dx5, dy5, dx6]) => {
                self.builder.curve_by([dx1, dy1, dx2, dy2, dx3, 0.0]);
                self.builder
                    .curve_by([dx4, 0.0, dx5, dy5, dx6, -(dy1 + dy2 + dy5)]);
            }
            (37, &[dx1, dy1, dx2, dy2, dx3, dy3, dx4, dy4, dx5, dy5, d6]) => {
                let dx = dx1 + dx2 + dx3 + dx4 + dx5;
                let dy = dy1 + dy2 + dy3 + dy4 + dy5;
                let (dx6, dy6) = if dx.abs() > dy.abs() {
                    (d6, -dy)
                } else {
                    (-dx, d6)
                };
                self.builder.curve_by([dx1, dy1, dx2, dy2, dx3, dy3]);
                self.builder.curve_by([dx4, dy4, dx5, dy5, dx6, dy6]);
            }
            _ => {
                return Err(FontError::malformed(
                    CFF,
                    "unsupported escaped charstring operator",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::{Midpoint, OutlineRecorder, PathCommand::*};

    fn run(code: &[u8], local: Index<'_>) -> (Vec<crate::outline::PathCommand>, Option<Seac>) {
        let mut outline = Outline::default();
        let seac = interpret(
            code,
            Subrs {
                global: Index::default(),
                local,
            },
            &mut outline,
        )
        .unwrap();
        let mut recorder = OutlineRecorder::default();
        outline.decompose(&mut recorder, Midpoint::Exact);
        (recorder.commands, seac)
    }

    /// Encodes small integers in charstring form.
    fn n(v: i32) -> u8 {
        (v + 139) as u8
    }

    #[test]
    fn square_with_width_and_closing_line() {
        // width 50, rmoveto 10 10, hlineto 20, vlineto 20, hlineto -20, endchar
        let code = [n(50), n(10), n(10), 21, n(20), 6, n(20), 7, n(-20), 6, 14];
        let (cmds, seac) = run(&code, Index::default());
        assert_eq!(
            cmds,
            vec![
                MoveTo(10.0, 10.0),
                LineTo(30.0, 10.0),
                LineTo(30.0, 30.0),
                LineTo(10.0, 30.0),
                LineTo(10.0, 10.0),
                Close
            ]
        );
        assert_eq!(seac, None);
    }

    #[test]
    fn curves_and_duplicate_end_point() {
        // rmoveto 0 0; rrcurveto to (30, 0); rlineto back to the start.
        let code = [
            n(0),
            n(0),
            21,
            n(10),
            n(10),
            n(10),
            n(0),
            n(10),
            n(-10),
            8,
            n(-30),
            n(0),
            5,
            14,
        ];
        let (cmds, _) = run(&code, Index::default());
        assert_eq!(
            cmds,
            vec![
                MoveTo(0.0, 0.0),
                CurveTo(10.0, 10.0, 20.0, 10.0, 30.0, 0.0),
                LineTo(0.0, 0.0),
                Close
            ]
        );
    }

    #[test]
    fn local_subroutines_and_hints() {
        // Subr 0 (biased -107): rlineto 5 5, return.
        let subr = [n(5), n(5), 5, 11];
        let mut index_bytes = vec![0, 1, 1, 1, 5];
        index_bytes.extend_from_slice(&subr);
        let local = Index::read(&mut Reader::new(&index_bytes, CFF)).unwrap();
        // hstem 0 10, hintmask (1 byte), rmoveto 1 1, callsubr -107, endchar
        let code = [n(0), n(10), 1, 19, 0xFF, n(1), n(1), 21, n(-107), 10, 14];
        let (cmds, _) = run(&code, local);
        assert_eq!(
            cmds,
            vec![MoveTo(1.0, 1.0), LineTo(6.0, 6.0), LineTo(1.0, 1.0), Close]
        );
    }

    #[test]
    fn seac_and_errors() {
        let code = [n(0), n(10), n(20), n(65), n(66), 14];
        let (_, seac) = run(&code, Index::default());
        assert_eq!(
            seac,
            Some(Seac {
                dx: 10.0,
                dy: 20.0,
                base: 65,
                accent: 66
            })
        );
        let mut outline = Outline::default();
        let subrs = Subrs {
            global: Index::default(),
            local: Index::default(),
        };
        assert!(interpret(&[n(0), 10], subrs, &mut outline).is_err());
        assert!(interpret(&[2], subrs, &mut outline).is_err());
        assert!(interpret(&[28, 1], subrs, &mut outline).is_err());
        assert!(interpret(&[n(1); 49], subrs, &mut outline).is_err());
    }

    #[test]
    fn single_point_contours_vanish() {
        let code = [n(5), n(5), 21, n(1), n(1), 21, n(3), 6, 14];
        let (cmds, _) = run(&code, Index::default());
        assert_eq!(
            cmds,
            vec![MoveTo(6.0, 6.0), LineTo(9.0, 6.0), LineTo(6.0, 6.0), Close]
        );
    }

    #[test]
    fn flex_variants() {
        let mut code = vec![n(0), n(0), 21];
        code.extend([n(1), n(2), n(3), n(4), n(5), n(6), n(7), 12, 34, 14]);
        let (cmds, _) = run(&code, Index::default());
        assert_eq!(cmds[1], CurveTo(1.0, 0.0, 3.0, 3.0, 7.0, 3.0));
        assert_eq!(cmds[2], CurveTo(12.0, 3.0, 18.0, 0.0, 25.0, 0.0));
        let mut outline = Outline::default();
        let subrs = Subrs {
            global: Index::default(),
            local: Index::default(),
        };
        assert!(interpret(&[n(1), n(2), 12, 34], subrs, &mut outline).is_err());
    }

    #[test]
    fn alternating_curves_with_final_extra() {
        // hvcurveto with 5 arguments: dx1 dx2 dy2 dy3 dxf.
        let code = [n(0), n(0), 21, n(10), n(5), n(5), n(10), n(2), 31, 14];
        let (cmds, _) = run(&code, Index::default());
        assert_eq!(cmds[1], CurveTo(10.0, 0.0, 15.0, 5.0, 17.0, 15.0));
    }
}
