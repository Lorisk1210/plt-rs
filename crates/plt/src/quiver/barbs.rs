//! Wind barb outlines (matplotlib's `Barbs._find_tails` and
//! `Barbs._make_barbs`).

use plt_core::numeric::np_divmod;
use plt_core::{Affine2D, Point};

use crate::collections::CollectionProps;
use crate::colors::Color;
use crate::transforms::Transform;

/// Proportions of barb features relative to the staff length
/// (matplotlib's `sizes`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarbSizes {
    /// Space between features (flags, full and half barbs).
    pub spacing: f64,
    /// Distance from the staff to the tip of a flag or full barb.
    pub height: f64,
    /// Width of a flag, twice the width of a full barb.
    pub width: f64,
    /// Radius of the circle drawn for magnitudes below a half barb.
    pub emptybarb: f64,
}

impl Default for BarbSizes {
    fn default() -> Self {
        Self {
            spacing: 0.125,
            height: 0.4,
            width: 0.25,
            emptybarb: 0.15,
        }
    }
}

/// The magnitudes a half barb, a full barb and a flag stand for
/// (matplotlib's `barb_increments`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarbIncrements {
    /// Magnitude of a half barb.
    pub half: f64,
    /// Magnitude of a full barb.
    pub full: f64,
    /// Magnitude of a flag.
    pub flag: f64,
}

impl Default for BarbIncrements {
    fn default() -> Self {
        Self {
            half: 5.0,
            full: 10.0,
            flag: 50.0,
        }
    }
}

/// The point of a barb placed on its position (matplotlib's `pivot`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BarbPivot {
    /// The end of the staff.
    #[default]
    Tip,
    /// The middle of the staff.
    Middle,
    /// The staff end shifted by this many staff-length units along the
    /// staff.
    Offset(f64),
}

/// Options of `Axes::barbs` (matplotlib's `barbs` keywords).
#[derive(Clone, Debug, PartialEq)]
pub struct BarbsOptions {
    /// The point of each barb placed on its position.
    pub pivot: BarbPivot,
    /// Staff length in points.
    pub length: f64,
    /// Colour of the staff and barbs (the polygon edges).
    pub barbcolor: Option<Color>,
    /// Colour of the flags (the polygon faces).
    pub flagcolor: Option<Color>,
    /// Feature proportions.
    pub sizes: BarbSizes,
    /// Fill the circles drawn for small magnitudes.
    pub fill_empty: bool,
    /// What each feature stands for.
    pub increments: BarbIncrements,
    /// Round magnitudes to the nearest half barb instead of truncating.
    pub rounding: bool,
    /// Draw the features on the other side of the staff, one flag per
    /// barb or one for all.
    pub flip: Vec<bool>,
    /// Further collection properties (face colour, edge width, ...).
    pub props: CollectionProps,
    /// The coordinates of the positions (`None` = the axes' data
    /// coordinates; matplotlib's `transform`), counted for the data limits
    /// as [`QuiverOptions::transform`](super::QuiverOptions::transform)
    /// says.
    pub transform: Option<Transform>,
}

impl Default for BarbsOptions {
    fn default() -> Self {
        Self {
            pivot: BarbPivot::Tip,
            length: 7.0,
            barbcolor: None,
            flagcolor: None,
            sizes: BarbSizes::default(),
            fill_empty: false,
            increments: BarbIncrements::default(),
            rounding: true,
            flip: vec![false],
            props: CollectionProps::default(),
            transform: None,
        }
    }
}

/// How many features one magnitude gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tail {
    flags: usize,
    barbs: usize,
    half: bool,
}

impl Tail {
    /// The features of a magnitude (matplotlib's `_find_tails`); `None`
    /// when it is below a half barb.
    fn of(magnitude: f64, rounding: bool, inc: &BarbIncrements) -> Option<Self> {
        let magnitude = if rounding {
            inc.half * (magnitude / inc.half).round_ties_even()
        } else {
            magnitude
        };
        let (flags, rest) = np_divmod(magnitude, inc.flag);
        let (barbs, rest) = np_divmod(rest, inc.full);
        let half = rest >= inc.half;
        // matplotlib decides emptiness on the float counts, then draws
        // `range(count.astype(int))` features.
        (half || flags > 0.0 || barbs > 0.0).then(|| Self {
            flags: feature_count(flags),
            barbs: feature_count(barbs),
            half,
        })
    }
}

/// How many features `range(count.astype(int))` draws: numpy's cast to
/// `int64` truncates, and turns NaN and counts out of its range into
/// `i64::MIN` (as x86-64 does), for which there are none.
fn feature_count(count: f64) -> usize {
    // 2^63, the first float above the range of `i64`.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    if (-LIMIT..LIMIT).contains(&count) {
        // Truncation, exact within the range.
        usize::try_from(count as i64).unwrap_or(0)
    } else {
        0
    }
}

/// Which side of the staff the features of each barb are drawn on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BarbFlip<'a>(pub(crate) &'a [bool]);

impl BarbFlip<'_> {
    fn get(&self, i: usize) -> bool {
        self.0[if self.0.len() == 1 { 0 } else { i }]
    }
}

/// The barb polygons of the vectors `(u, v)` in staff-length units,
/// rotated to point into the wind (matplotlib's `_make_barbs`).
pub(crate) fn barb_outlines(
    uv: (&[f64], &[f64]),
    options: &BarbsOptions,
    flip: BarbFlip<'_>,
) -> Vec<Vec<Point>> {
    let length = options.length;
    let sizes = &options.sizes;
    let spacing = length * sizes.spacing;
    let full_height = length * sizes.height;
    let full_width = length * sizes.width;
    let empty = empty_barb(length * sizes.emptybarb, options.fill_empty);
    let end_y = match options.pivot {
        BarbPivot::Tip => 0.0,
        BarbPivot::Middle => -length / 2.0,
        BarbPivot::Offset(y) => y,
    };
    let (u, v) = uv;
    u.iter()
        .zip(v)
        .enumerate()
        .map(|(i, (&u, &v))| {
            let Some(tail) = Tail::of(u.hypot(v), options.rounding, &options.increments) else {
                return empty.clone();
            };
            let height = if flip.get(i) {
                -full_height
            } else {
                full_height
            };
            let staff = Staff {
                end_y,
                length,
                spacing,
                height,
                width: full_width,
            };
            // The barb is drawn along +y (north); rotate it into the wind.
            let angle = -(v.atan2(u) + std::f64::consts::FRAC_PI_2);
            let rotation = Affine2D::rotation(-angle);
            staff
                .outline(tail)
                .into_iter()
                .map(|p| rotation.transform_point(p))
                .collect()
        })
        .collect()
}

/// The circle of a magnitude below a half barb: a polygon, or (unfilled)
/// one that runs back over itself.
fn empty_barb(radius: f64, fill: bool) -> Vec<Point> {
    let circle = circle_polygon(radius);
    if fill {
        circle
    } else {
        circle.iter().chain(circle.iter().rev()).copied().collect()
    }
}

/// The vertices of matplotlib's `CirclePolygon((0, 0), radius)`: a
/// 20-gon starting at the top, closed by repeating its first vertex.
fn circle_polygon(radius: f64) -> Vec<Point> {
    const SIDES: usize = 20;
    let mut vertices: Vec<Point> = (0..SIDES)
        .map(|i| {
            let theta =
                2.0 * std::f64::consts::PI / SIDES as f64 * i as f64 + std::f64::consts::FRAC_PI_2;
            Point::new(theta.cos() * radius, theta.sin() * radius)
        })
        .collect();
    vertices.push(vertices[0]);
    vertices
}

/// The dimensions of one barb staff, in points before scaling.
struct Staff {
    end_y: f64,
    length: f64,
    spacing: f64,
    /// Signed distance from the staff to a feature tip (negative when
    /// flipped).
    height: f64,
    width: f64,
}

impl Staff {
    /// The polygon of a staff with `tail`'s features, drawn along +y.
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib tells the first feature by the exact staff offset"
    )]
    fn outline(&self, tail: Tail) -> Vec<Point> {
        let (x, y) = (0.0, self.end_y);
        let on_staff = |offset: f64| Point::new(x, y + offset);
        let mut vertices = vec![Point::new(x, y)];
        let mut offset = self.length;
        for _ in 0..tail.flags {
            // Flags are a little closer together than barbs.
            if offset != self.length {
                offset += self.spacing / 2.0;
            }
            vertices.extend([
                on_staff(offset),
                Point::new(x + self.height, y - self.width / 2.0 + offset),
                Point::new(x, y - self.width + offset),
            ]);
            offset -= self.width + self.spacing;
        }
        for _ in 0..tail.barbs {
            vertices.extend([
                on_staff(offset),
                Point::new(x + self.height, y + offset + self.width / 2.0),
                on_staff(offset),
            ]);
            offset -= self.spacing;
        }
        if tail.half {
            // A lone half barb sits away from the end, to tell it from a
            // full one.
            if offset == self.length {
                vertices.push(on_staff(offset));
                offset -= 1.5 * self.spacing;
            }
            vertices.extend([
                on_staff(offset),
                Point::new(x + self.height / 2.0, y + offset + self.width / 4.0),
                on_staff(offset),
            ]);
        }
        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail(magnitude: f64, rounding: bool) -> Option<Tail> {
        Tail::of(magnitude, rounding, &BarbIncrements::default())
    }

    #[test]
    fn magnitudes_split_into_flags_barbs_and_halves() {
        assert_eq!(tail(0.0, true), None);
        assert_eq!(tail(2.4, true), None);
        assert_eq!(
            tail(3.0, true),
            Some(Tail {
                flags: 0,
                barbs: 0,
                half: true
            })
        );
        assert_eq!(
            tail(65.0, true),
            Some(Tail {
                flags: 1,
                barbs: 1,
                half: true
            })
        );
        assert_eq!(
            tail(140.0, true),
            Some(Tail {
                flags: 2,
                barbs: 4,
                half: false
            })
        );
        assert_eq!(tail(3.0, false), None, "truncated below a half barb");
        assert_eq!(tail(f64::NAN, true), None);
    }

    #[test]
    fn huge_magnitudes_draw_no_flags() {
        // numpy's astype(int) of 2e298 flags is i64::MIN: range() is empty,
        // but the barb is not, as its float count is positive.
        let huge = tail(1e300, true).expect("not empty");
        assert_eq!(huge.flags, 0);
        assert_eq!(feature_count(f64::NAN), 0);
        assert_eq!(feature_count(-3.0), 0);
        assert_eq!(feature_count(3.9), 3);
    }

    #[test]
    fn rounding_is_to_even_halves() {
        // 7.5 / 5 = 1.5 rounds to 2 (numpy's around), giving a full barb.
        assert_eq!(
            tail(7.5, true),
            Some(Tail {
                flags: 0,
                barbs: 1,
                half: false
            })
        );
        assert_eq!(
            tail(12.5, true),
            Some(Tail {
                flags: 0,
                barbs: 1,
                half: false
            })
        );
    }

    #[test]
    fn staff_features() {
        let options = BarbsOptions::default();
        let staff = Staff {
            end_y: 0.0,
            length: 7.0,
            spacing: 7.0 * 0.125,
            height: 7.0 * 0.4,
            width: 7.0 * 0.25,
        };
        let half = staff.outline(Tail {
            flags: 0,
            barbs: 0,
            half: true,
        });
        assert_eq!(half.len(), 5, "start, offset point and the half barb");
        assert_eq!(half[2].y, 7.0 - 1.5 * 0.875);
        let flags = staff.outline(Tail {
            flags: 2,
            barbs: 1,
            half: false,
        });
        assert_eq!(flags.len(), 1 + 6 + 3);
        let outlines = barb_outlines((&[0.0], &[-10.0]), &options, BarbFlip(&[false]));
        // Wind from the north: the staff points up.
        assert!(outlines[0][1].y > 6.9 && outlines[0][1].x.abs() < 1e-12);
    }

    #[test]
    fn empty_barbs_are_circles() {
        let options = BarbsOptions::default();
        let outlines = barb_outlines((&[0.0], &[0.0]), &options, BarbFlip(&[false]));
        assert_eq!(outlines[0].len(), 42, "circle and its reverse");
        let filled = BarbsOptions {
            fill_empty: true,
            ..options
        };
        let outlines = barb_outlines((&[0.0], &[0.0]), &filled, BarbFlip(&[false]));
        assert_eq!(outlines[0].len(), 21);
        assert!((outlines[0][0].y - 7.0 * 0.15).abs() < 1e-15);
    }
}
