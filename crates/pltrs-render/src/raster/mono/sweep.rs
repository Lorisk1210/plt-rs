//! The scanline sweep (`ftraster.c`'s `Draw_Sweep`): pairs the ascending
//! and descending profiles crossing each scanline, fills the pixels whose
//! centres lie between them, and applies drop-out control where a pair
//! covers no pixel centre. The vertical pass sweeps rows; the horizontal
//! pass sweeps columns, adding only perfectly aligned edges and the
//! drop-outs the rows missed.

use super::MonoBitmap;
use super::profiles::{
    DROPOUT, FLOW_UP, OVERSHOOT_BOTTOM, OVERSHOOT_TOP, Precision, Profile, Profiles,
};

/// Which way a sweep runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pass {
    /// Scanlines are rows; crossings are x positions.
    Vertical,
    /// Scanlines are columns; crossings are y positions (the outline was
    /// converted with x and y swapped).
    Horizontal,
}

impl Pass {
    /// The pixel (column, row from the bottom) at crossing pixel `e` of
    /// scanline `y`.
    const fn pixel(self, e: i64, y: i64) -> (i64, i64) {
        match self {
            Self::Vertical => (e, y),
            Self::Horizontal => (y, e),
        }
    }
}

/// `Draw_Sweep` over the profiles of one band.
pub(super) fn draw_sweep(
    profiles: &mut Profiles,
    precision: Precision,
    pass: Pass,
    bitmap: &mut MonoBitmap,
) {
    let (Some(&min_y), Some(&top)) = (profiles.turns.first(), profiles.turns.last()) else {
        return;
    };
    let max_y = top - 1;
    let mut waiting: Vec<usize> = (0..profiles.list.len()).collect();
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let turns = profiles.turns.clone();
    let mut turns = turns[1..].iter().copied();
    let mut y = min_y;
    while y <= max_y {
        waiting.retain(|&index| {
            let profile = &profiles.list[index];
            if profile.start != y {
                return true;
            }
            let list = if profile.flags & FLOW_UP != 0 {
                &mut left
            } else {
                &mut right
            };
            insert_sorted(list, &profiles.list, index);
            false
        });
        let Some(y_turn) = turns.next() else {
            return;
        };
        loop {
            let dropouts = trace(profiles, precision, &left, &right, |x1, x2| {
                span(bitmap, precision, pass, y, x1, x2);
            });
            if dropouts > 0 {
                for (&l, &r) in left.iter().zip(&right) {
                    if profiles.list[l].flags & DROPOUT != 0 {
                        let (x1, x2) = (profiles.list[l].x, profiles.list[r].x);
                        drop_out(bitmap, precision, pass, y, x1, x2);
                        profiles.list[l].flags &= !DROPOUT;
                    }
                }
            }
            increment(profiles, &mut left, 1);
            increment(profiles, &mut right, -1);
            y += 1;
            if y >= y_turn {
                break;
            }
        }
    }
}

/// Fills the spans between each pair of profiles at the current scanline
/// and marks the drop-outs, returning how many there are.
fn trace(
    profiles: &mut Profiles,
    precision: Precision,
    left: &[usize],
    right: &[usize],
    mut fill: impl FnMut(i64, i64),
) -> usize {
    let mut dropouts = 0;
    for (&l, &r) in left.iter().zip(right) {
        let (mut x1, mut x2) = (profiles.list[l].x, profiles.list[r].x);
        // TrueType's contours put x1 first, but others may not.
        if x1 > x2 {
            (x1, x2) = (x2, x1);
        }
        if precision.ceiling(x1) <= precision.floor(x2) {
            fill(x1, x2);
            continue;
        }
        let (left_profile, right_profile) = (profiles.list[l], profiles.list[r]);
        let Some((x1, x2)) =
            dropout_pixels(precision, &left_profile, &right_profile, (l, r), x1, x2)
        else {
            continue;
        };
        profiles.list[l].x = x2;
        profiles.list[r].x = x1;
        profiles.list[l].flags |= DROPOUT;
        dropouts += 1;
    }
    dropouts
}

/// The drop-out control of a pair covering no pixel centre: `None` when
/// it stays empty, else the pixel to set and the other one, as the
/// `(right, left)` crossings to record.
fn dropout_pixels(
    precision: Precision,
    left: &Profile,
    right: &Profile,
    (l, r): (usize, usize),
    x1: i64,
    x2: i64,
) -> Option<(i64, i64)> {
    let control = left.flags & 7;
    if control & 2 != 0 {
        return None;
    }
    if control & 1 != 0 {
        let wide = x2 - x1 >= precision.half();
        let upper_stub = left.height == 1 && left.next == r;
        if upper_stub && !(left.flags & OVERSHOOT_TOP != 0 && wide) {
            return None;
        }
        let lower_stub = left.offset == 0 && right.next == l;
        if lower_stub && !(left.flags & OVERSHOOT_BOTTOM != 0 && wide) {
            return None;
        }
    }
    Some(if control & 4 != 0 {
        let x2 = precision.smart(x1, x2);
        let other = if x1 > x2 {
            x2 + precision.one()
        } else {
            x2 - precision.one()
        };
        (other, x2)
    } else {
        (precision.ceiling(x1), precision.floor(x2))
    })
}

/// `InsNew`: inserts `index` before the first profile at or right of it.
fn insert_sorted(list: &mut Vec<usize>, profiles: &[Profile], index: usize) {
    let x = profiles[index].x;
    let at = list
        .iter()
        .position(|&other| profiles[other].x >= x)
        .unwrap_or(list.len());
    list.insert(at, index);
}

/// `Increment`: moves the profiles of `list` to the next scanline, drops
/// the exhausted ones and keeps the list sorted.
fn increment(profiles: &mut Profiles, list: &mut Vec<usize>, flow: i64) {
    list.retain(|&index| {
        let profile = profiles.list[index];
        if profile.height <= 1 {
            profiles.list[index].height = 0;
            return false;
        }
        let offset = profile.offset + flow;
        let x = profiles.crossing(&profile, offset);
        let profile = &mut profiles.list[index];
        profile.height -= 1;
        profile.offset = offset;
        profile.x = x;
        true
    });
    // FreeType's adjacent swaps leave equal profiles in order: a stable
    // sort.
    list.sort_by_key(|&index| profiles.list[index].x);
}

/// `Vertical_Sweep_Span` and `Horizontal_Sweep_Span`.
fn span(bitmap: &mut MonoBitmap, p: Precision, pass: Pass, y: i64, x1: i64, x2: i64) {
    match pass {
        Pass::Vertical => {
            let e1 = p.trunc(p.ceiling(x1)).max(0);
            let e2 = p.trunc(p.floor(x2)).min(bitmap.right());
            for x in e1..=e2 {
                bitmap.set(x, y);
            }
        }
        // The rows mishandle horizontal edges through pixel centres, so
        // the columns add the aligned ends.
        Pass::Horizontal => {
            for (edge, aligned) in [(x1, p.ceiling(x1)), (x2, p.floor(x2))] {
                let row = p.trunc(aligned);
                if edge == aligned && (0..=bitmap.top()).contains(&row) {
                    bitmap.set(y, row);
                }
            }
        }
    }
}

/// `Vertical_Sweep_Drop` and `Horizontal_Sweep_Drop`: sets pixel `x1`
/// unless pixel `x2` already is; a pixel outside the bitmap yields to
/// the other.
fn drop_out(bitmap: &mut MonoBitmap, p: Precision, pass: Pass, y: i64, x1: i64, x2: i64) {
    let (e1, e2) = (p.trunc(x1), p.trunc(x2));
    let limit = match pass {
        Pass::Vertical => bitmap.right(),
        Pass::Horizontal => bitmap.top(),
    };
    let inside = |e: i64| (0..=limit).contains(&e);
    let pixel = if inside(e1) {
        let (x, row) = pass.pixel(e2, y);
        if inside(e2) && bitmap.get(x, row) {
            return;
        }
        e1
    } else {
        e2
    };
    if inside(pixel) {
        let (x, row) = pass.pixel(pixel, y);
        bitmap.set(x, row);
    }
}
