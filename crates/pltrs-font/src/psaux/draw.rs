//! The drawing operators of `cf2_interpT2CharString`: moves, lines and
//! curves, the stem declarations, and Type 1's `closepath` and `hsbw`.

use super::error::CharstringError;
use super::interp::{Axis, Flow, Invocation, Pass, add};
use super::path::Point;

impl Invocation<'_, '_> {
    fn line_to(&mut self, x: i32, y: i32) -> Result<(), CharstringError> {
        self.cur = (x, y);
        self.path.line_to(x, y, &mut self.decoder.out)
    }

    pub(super) fn curve_to(&mut self, points: [Point; 3]) -> Result<(), CharstringError> {
        self.cur = points[2];
        self.path.curve_to(points, &mut self.decoder.out)
    }

    /// A line from the current point by the two deltas at `at`.
    fn relative_line(&mut self, at: usize) -> Result<(), CharstringError> {
        let (dx, dy) = (self.stack.get(at)?, self.stack.get(at + 1)?);
        self.line_to(add(self.cur.0, dx), add(self.cur.1, dy))
    }

    /// A curve from the current point by the six deltas at `at`.
    fn relative_curve(&mut self, at: usize) -> Result<(), CharstringError> {
        let get = |i| self.stack.get(at + i);
        let p1 = (add(get(0)?, self.cur.0), add(get(1)?, self.cur.1));
        let p2 = (add(get(2)?, p1.0), add(get(3)?, p1.1));
        let p3 = (add(get(4)?, p2.0), add(get(5)?, p2.1));
        self.curve_to([p1, p2, p3])
    }

    /// `rmoveto`, `hmoveto` and `vmoveto`; during a flex the moves only
    /// collect its points.
    fn move_by(&mut self, dx: i32, dy: i32) -> Result<Flow, CharstringError> {
        self.have_width = true;
        self.cur = (add(self.cur.0, dx), add(self.cur.1, dy));
        if !self.decoder.flex_state {
            self.path
                .move_to(self.cur.0, self.cur.1, &mut self.decoder.out)?;
        }
        Ok(Flow::Clear)
    }

    /// The one-byte drawing, hinting and width operators.
    pub(super) fn draw(&mut self, op: u8) -> Result<Flow, CharstringError> {
        let count = self.stack.count();
        match op {
            1 | 18 => return self.declare_stems(Axis::Horizontal),
            3 | 23 => return self.declare_stems(Axis::Vertical),
            4 => {
                let dy = self.stack.pop_fixed()?;
                return self.move_by(0, dy);
            }
            21 => {
                let dy = self.stack.pop_fixed()?;
                let dx = self.stack.pop_fixed()?;
                return self.move_by(dx, dy);
            }
            22 => {
                let dx = self.stack.pop_fixed()?;
                return self.move_by(dx, 0);
            }
            5 => {
                for i in (0..count).step_by(2) {
                    self.relative_line(i)?;
                }
            }
            6 | 7 => {
                let mut horizontal = op == 6;
                for i in 0..count {
                    let d = self.stack.get(i)?;
                    let (x, y) = self.cur;
                    if horizontal {
                        self.line_to(add(x, d), y)?;
                    } else {
                        self.line_to(x, add(y, d))?;
                    }
                    horizontal = !horizontal;
                }
            }
            8 | 24 => {
                let mut i = 0;
                while i + 6 <= count {
                    self.relative_curve(i)?;
                    i += 6;
                }
                if op == 24 {
                    self.relative_line(i)?;
                }
            }
            25 => {
                let mut i = 0;
                while i + 6 < count {
                    self.relative_line(i)?;
                    i += 2;
                }
                while i < count {
                    self.relative_curve(i)?;
                    i += 6;
                }
            }
            26 | 27 => self.flat_curves(op == 27, count)?,
            30 | 31 => self.alternating_curves(op == 31, count)?,
            9 if self.is_type1() => {
                self.path.close_open_path(&mut self.decoder.out)?;
                self.have_width = true;
                return Ok(Flow::Clear);
            }
            13 if self.is_type1() => return self.hsbw(),
            // Reserved, and Type 1's `closepath` and `hsbw` elsewhere.
            _ => return Ok(Flow::Clear),
        }
        self.stack.clear();
        Ok(Flow::Keep)
    }

    /// `hsbw`: the side bearing and the advance.
    fn hsbw(&mut self) -> Result<Flow, CharstringError> {
        let advance = self.stack.pop_fixed()?;
        self.decoder.advance = (advance, 0);
        let lsb = self.stack.pop_fixed()?;
        self.decoder.left_bearing.0 = add(self.decoder.left_bearing.0, lsb);
        self.have_width = true;
        if self.pass == Pass::Draw {
            self.cur.0 = add(self.cur.0, lsb);
        }
        Ok(Flow::Clear)
    }

    /// `vvcurveto` and `hhcurveto`: an optional leading delta across, then
    /// groups of four.
    fn flat_curves(&mut self, horizontal: bool, count1: usize) -> Result<(), CharstringError> {
        let count = count1 & !2;
        let mut i = count1 - count;
        while i < count {
            let lead = if (count - i) & 1 == 1 {
                i += 1;
                Some(self.stack.get(i - 1)?)
            } else {
                None
            };
            let get = |k| self.stack.get(i + k);
            let (x, y) = self.cur;
            let points = if horizontal {
                let y1 = lead.map_or(y, |d| add(d, y));
                let p1 = (add(get(0)?, x), y1);
                let p2 = (add(get(1)?, p1.0), add(get(2)?, y1));
                [p1, p2, (add(get(3)?, p2.0), p2.1)]
            } else {
                let x1 = lead.map_or(x, |d| add(d, x));
                let p1 = (x1, add(get(0)?, y));
                let p2 = (add(get(1)?, x1), add(get(2)?, p1.1));
                [p1, p2, (p2.0, add(get(3)?, p2.1))]
            };
            self.curve_to(points)?;
            i += 4;
        }
        Ok(())
    }

    /// `hvcurveto` and `vhcurveto`: curves alternating between horizontal
    /// and vertical tangents; a fifth value bends the last one's end.
    fn alternating_curves(
        &mut self,
        mut horizontal: bool,
        count1: usize,
    ) -> Result<(), CharstringError> {
        let count = count1 & !2;
        let mut i = count1 - count;
        while i < count {
            let get = |k| self.stack.get(i + k);
            let (x, y) = self.cur;
            let last = count - i == 5;
            let points = if horizontal {
                let p1 = (add(get(0)?, x), y);
                let p2 = (add(get(1)?, p1.0), add(get(2)?, p1.1));
                let y3 = add(get(3)?, p2.1);
                let x3 = if last { add(get(4)?, p2.0) } else { p2.0 };
                [p1, p2, (x3, y3)]
            } else {
                let p1 = (x, add(get(0)?, y));
                let p2 = (add(get(1)?, p1.0), add(get(2)?, p1.1));
                let x3 = add(get(3)?, p2.0);
                let y3 = if last { add(get(4)?, p2.1) } else { p2.1 };
                [p1, p2, (x3, y3)]
            };
            if last {
                i += 1;
            }
            horizontal = !horizontal;
            self.curve_to(points)?;
            i += 4;
        }
        Ok(())
    }
}
