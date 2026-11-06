//! Reading SVG path data (the `d` attribute of `<path>`) into a [`Path`].
//!
//! The grammar is SVG 1.1's: absolute (upper case) and relative (lower
//! case) moves, lines (`L`, `H`, `V`), cubic (`C`, `S`) and quadratic
//! (`Q`, `T`) Béziers and closures (`Z`), numbers separated by white
//! space or commas or by nothing where a sign or a second decimal point
//! ends one (`M0-1.5.5`), and repeated coordinates continuing the last
//! command (a move's further pairs are lines). Elliptical arcs (`A`) are
//! refused.

use std::fmt;

use super::{Path, PathBuilder};
use crate::Point;

/// Why SVG path data could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SvgPathError {
    /// A character that is neither a command, a number nor a separator,
    /// at a byte offset.
    UnexpectedCharacter {
        /// Byte offset in the data.
        offset: usize,
        /// The character found.
        found: char,
    },
    /// A command this reader does not draw (elliptical arcs).
    UnsupportedCommand(char),
    /// Data that does not start with a move.
    MissingInitialMove,
    /// A command whose coordinates stop short, at a byte offset.
    MissingNumber {
        /// The command.
        command: char,
        /// Byte offset where a number was expected.
        offset: usize,
    },
}

impl fmt::Display for SvgPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter { offset, found } => {
                write!(f, "unexpected {found:?} at byte {offset} of SVG path data")
            }
            Self::UnsupportedCommand(command) => {
                write!(f, "SVG path command {command:?} is not supported")
            }
            Self::MissingInitialMove => f.write_str("SVG path data must start with a move"),
            Self::MissingNumber { command, offset } => write!(
                f,
                "SVG path command {command:?} lacks a number at byte {offset}"
            ),
        }
    }
}

impl std::error::Error for SvgPathError {}

impl Path {
    /// Reads SVG path data, as in `<path d="M0 0H10V10Z"/>`: SVG 1.1's
    /// grammar, absolute and relative moves, lines (`L`, `H`, `V`), cubic
    /// (`C`, `S`) and quadratic (`Q`, `T`) Béziers and closures (`Z`),
    /// with numbers run together where a sign or a second decimal point
    /// separates them (`M0-1.5.5`). Elliptical arcs (`A`) are refused.
    ///
    /// ```
    /// use pltrs_core::{Path, Point};
    /// let square = Path::from_svg("M0 0h10v10H0z")?;
    /// assert_eq!(square.vertices()[2], Point::new(10.0, 10.0));
    /// # Ok::<(), pltrs_core::path::SvgPathError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the data does not follow the grammar, does not start with a
    /// move, or has an arc.
    pub fn from_svg(data: &str) -> Result<Self, SvgPathError> {
        let mut reader = Reader {
            scanner: Scanner { data, offset: 0 },
            builder: Path::builder(),
            current: Point::ORIGIN,
            start: Point::ORIGIN,
            reflected: None,
        };
        let mut command = None;
        while let Some(next) = reader.scanner.next_token()? {
            let letter = match next {
                Token::Command(letter) => {
                    reader.scanner.offset += 1;
                    letter
                }
                // Coordinates repeat the last command; after a move,
                // they are lines.
                Token::Number => match command {
                    Some('M') => 'L',
                    Some('m') => 'l',
                    Some(letter) if !letter.eq_ignore_ascii_case(&'z') => letter,
                    None => return Err(SvgPathError::MissingInitialMove),
                    Some(_) => {
                        return Err(SvgPathError::UnexpectedCharacter {
                            offset: reader.scanner.offset,
                            found: reader.scanner.peek().unwrap_or(' '),
                        });
                    }
                },
            };
            if command.is_none() && !letter.eq_ignore_ascii_case(&'m') {
                return Err(SvgPathError::MissingInitialMove);
            }
            reader.command(letter)?;
            command = Some(letter);
        }
        Ok(reader.builder.build())
    }
}

/// What comes next in the data.
enum Token {
    /// A command letter (not yet consumed).
    Command(char),
    /// The start of a number.
    Number,
}

/// A position in path data.
struct Scanner<'a> {
    data: &'a str,
    offset: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.data[self.offset..].chars().next()
    }

    fn skip_separators(&mut self) {
        while let Some(c) = self.peek()
            && (c.is_ascii_whitespace() || c == ',')
        {
            self.offset += 1;
        }
    }

    /// Skips separators and classifies what follows; `None` at the end.
    fn next_token(&mut self) -> Result<Option<Token>, SvgPathError> {
        self.skip_separators();
        let Some(c) = self.peek() else {
            return Ok(None);
        };
        match c {
            'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q' | 'q'
            | 'T' | 't' | 'Z' | 'z' => Ok(Some(Token::Command(c))),
            'A' | 'a' => Err(SvgPathError::UnsupportedCommand(c)),
            '0'..='9' | '+' | '-' | '.' => Ok(Some(Token::Number)),
            found => Err(SvgPathError::UnexpectedCharacter {
                offset: self.offset,
                found,
            }),
        }
    }

    /// Reads a number after optional separators: sign, digits, fraction
    /// and exponent.
    fn number(&mut self, command: char) -> Result<f64, SvgPathError> {
        self.skip_separators();
        let bytes = self.data.as_bytes();
        let start = self.offset;
        let mut end = start;
        let digits = |mut at: usize| {
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            at
        };
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let integer = digits(end);
        let mut mantissa = integer > end;
        end = integer;
        if bytes.get(end) == Some(&b'.') {
            let fraction = digits(end + 1);
            mantissa |= fraction > end + 1;
            end = fraction;
        }
        if mantissa && matches!(bytes.get(end), Some(b'e' | b'E')) {
            let mut exponent = end + 1;
            if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            let exponent_end = digits(exponent);
            if exponent_end > exponent {
                end = exponent_end;
            }
        }
        let missing = SvgPathError::MissingNumber {
            command,
            offset: start,
        };
        if !mantissa {
            return Err(missing);
        }
        let value = self.data[start..end].parse().map_err(|_| missing)?;
        self.offset = end;
        Ok(value)
    }
}

/// The state of a path being read.
struct Reader<'a> {
    scanner: Scanner<'a>,
    builder: PathBuilder,
    /// The current point.
    current: Point,
    /// The start of the current sub-path, where closing returns.
    start: Point,
    /// The control point the last curve command leaves for a smooth one
    /// to reflect: cubic (`true`) or quadratic.
    reflected: Option<(Point, bool)>,
}

impl Reader<'_> {
    /// A point from two numbers, relative to the current point for lower
    /// case commands.
    fn point(&mut self, command: char) -> Result<Point, SvgPathError> {
        let x = self.scanner.number(command)?;
        let y = self.scanner.number(command)?;
        Ok(self.place(command, Point::new(x, y)))
    }

    fn place(&self, command: char, p: Point) -> Point {
        if command.is_ascii_lowercase() {
            Point::new(self.current.x + p.x, self.current.y + p.y)
        } else {
            p
        }
    }

    /// The reflection of the last curve's control point of the same
    /// kind, or the current point.
    fn reflection(&self, cubic: bool) -> Point {
        match self.reflected {
            Some((control, kind)) if kind == cubic => Point::new(
                2.0 * self.current.x - control.x,
                2.0 * self.current.y - control.y,
            ),
            _ => self.current,
        }
    }

    /// Reads one command's coordinates and adds its segment.
    fn command(&mut self, command: char) -> Result<(), SvgPathError> {
        let mut builder = std::mem::take(&mut self.builder);
        let mut reflected = None;
        let end = match command.to_ascii_uppercase() {
            'M' => {
                let p = self.point(command)?;
                self.start = p;
                builder = builder.move_to(p);
                p
            }
            'L' => {
                let p = self.point(command)?;
                builder = builder.line_to(p);
                p
            }
            'H' => {
                let x = self.scanner.number(command)?;
                let x = if command == 'h' {
                    self.current.x + x
                } else {
                    x
                };
                let p = Point::new(x, self.current.y);
                builder = builder.line_to(p);
                p
            }
            'V' => {
                let y = self.scanner.number(command)?;
                let y = if command == 'v' {
                    self.current.y + y
                } else {
                    y
                };
                let p = Point::new(self.current.x, y);
                builder = builder.line_to(p);
                p
            }
            'C' | 'S' => {
                let first = if command.eq_ignore_ascii_case(&'C') {
                    self.point(command)?
                } else {
                    self.reflection(true)
                };
                let second = self.point(command)?;
                let p = self.point(command)?;
                builder = builder.cubic_to(first, second, p);
                reflected = Some((second, true));
                p
            }
            'Q' | 'T' => {
                let control = if command.eq_ignore_ascii_case(&'Q') {
                    self.point(command)?
                } else {
                    self.reflection(false)
                };
                let p = self.point(command)?;
                builder = builder.quad_to(control, p);
                reflected = Some((control, false));
                p
            }
            _ => {
                builder = builder.close();
                self.start
            }
        };
        self.builder = builder;
        self.current = end;
        self.reflected = reflected;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PathCode::{ClosePoly, Curve3, Curve4, LineTo, MoveTo};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn absolute_and_relative_lines_close() {
        let path = Path::from_svg("M10 20 H30 V40 h-5 v-5 L0,0 l1 1 Z").unwrap();
        assert_eq!(
            path.vertices(),
            [
                p(10.0, 20.0),
                p(30.0, 20.0),
                p(30.0, 40.0),
                p(25.0, 40.0),
                p(25.0, 35.0),
                p(0.0, 0.0),
                p(1.0, 1.0),
                p(10.0, 20.0),
            ]
        );
        assert_eq!(
            path.codes().unwrap(),
            [
                MoveTo, LineTo, LineTo, LineTo, LineTo, LineTo, LineTo, ClosePoly
            ]
        );
    }

    #[test]
    fn repeated_pairs_after_a_move_are_lines() {
        let path = Path::from_svg("m1 1 2 0 0 2z m1 1 1 0").unwrap();
        assert_eq!(
            path.vertices(),
            [
                p(1.0, 1.0),
                p(3.0, 1.0),
                p(3.0, 3.0),
                p(1.0, 1.0),
                // A relative move after a closure starts from the
                // sub-path's start.
                p(2.0, 2.0),
                p(3.0, 2.0),
            ]
        );
        assert_eq!(
            path.codes().unwrap(),
            [MoveTo, LineTo, LineTo, ClosePoly, MoveTo, LineTo]
        );
    }

    #[test]
    fn numbers_need_no_separators() {
        // A sign or a second point starts a number; exponents take signs.
        let path = Path::from_svg("M0-1.5.5 1e1,2E-1+3").unwrap();
        assert_eq!(path.vertices(), [p(0.0, -1.5), p(0.5, 10.0), p(0.2, 3.0)]);
        // An exponent marker without digits ends the number before it.
        assert_eq!(
            Path::from_svg("M1e 2"),
            Err(SvgPathError::MissingNumber {
                command: 'M',
                offset: 2
            })
        );
    }

    #[test]
    fn cubic_and_smooth_cubic_curves() {
        let path = Path::from_svg("M0 0C1 0 2 1 2 2S3 4 4 4c1 0 1 1 1 1").unwrap();
        assert_eq!(
            path.vertices(),
            [
                p(0.0, 0.0),
                p(1.0, 0.0),
                p(2.0, 1.0),
                p(2.0, 2.0),
                // Reflects (2, 1) through (2, 2).
                p(2.0, 3.0),
                p(3.0, 4.0),
                p(4.0, 4.0),
                p(5.0, 4.0),
                p(5.0, 5.0),
                p(5.0, 5.0),
            ]
        );
        assert_eq!(
            path.codes().unwrap(),
            [
                MoveTo, Curve4, Curve4, Curve4, Curve4, Curve4, Curve4, Curve4, Curve4, Curve4
            ]
        );
    }

    #[test]
    fn quadratic_and_smooth_quadratic_curves() {
        let path = Path::from_svg("M0 0Q1 1 2 0T4 0t2 0").unwrap();
        assert_eq!(
            path.vertices(),
            [
                p(0.0, 0.0),
                p(1.0, 1.0),
                p(2.0, 0.0),
                p(3.0, -1.0),
                p(4.0, 0.0),
                p(5.0, 1.0),
                p(6.0, 0.0),
            ]
        );
        assert_eq!(
            path.codes().unwrap(),
            [MoveTo, Curve3, Curve3, Curve3, Curve3, Curve3, Curve3]
        );
        // A smooth curve without a curve of its kind before it takes the
        // current point as its control.
        let path = Path::from_svg("M0 0C1 1 2 1 3 0T5 0").unwrap();
        assert_eq!(path.vertices()[4], p(3.0, 0.0));
    }

    #[test]
    fn empty_data_is_an_empty_path() {
        assert!(Path::from_svg("").unwrap().is_empty());
        assert!(Path::from_svg(" \n, ").unwrap().is_empty());
    }

    #[test]
    fn malformed_data_is_refused() {
        assert_eq!(
            Path::from_svg("L1 1"),
            Err(SvgPathError::MissingInitialMove)
        );
        assert_eq!(Path::from_svg("1 1"), Err(SvgPathError::MissingInitialMove));
        assert_eq!(
            Path::from_svg("M0 0A1 1 0 0 1 2 2"),
            Err(SvgPathError::UnsupportedCommand('A'))
        );
        assert_eq!(
            Path::from_svg("M0 0 L1"),
            Err(SvgPathError::MissingNumber {
                command: 'L',
                offset: 7
            })
        );
        assert_eq!(
            Path::from_svg("M0 0 L1 1 x"),
            Err(SvgPathError::UnexpectedCharacter {
                offset: 10,
                found: 'x'
            })
        );
        assert_eq!(
            Path::from_svg("M0 0z 1 1"),
            Err(SvgPathError::UnexpectedCharacter {
                offset: 6,
                found: '1'
            })
        );
        assert!(Path::from_svg("M. 0").is_err());
        assert!(Path::from_svg("M- 0").is_err());
        assert!(Path::from_svg("M0 0 L1 é").is_err());
        assert!(
            SvgPathError::UnsupportedCommand('a')
                .to_string()
                .contains("'a'")
        );
    }
}
