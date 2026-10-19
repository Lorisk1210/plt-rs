//! Classifying the mesh for one trace: the z-value of every point (0, 1
//! or 2 against the levels), which edges are boundaries and where curves
//! may start (gcntr's `data_init` and `mask_zones`).
//!
//! Each index `ij` names a point, the zone whose upper right corner it is,
//! the i-edge running down from it (constant i) and the j-edge running
//! left of it (constant j). The data array has an extra row (and one) so
//! that the zones above the last row can be looked up.

use super::Mpl2005;
use crate::contour::quad::slot;

/// The z-value of the point (0, 1 or 2).
pub(super) const Z_VALUE: u16 = 0x0003;
/// The zone exists.
pub(super) const ZONE_EX: u16 = 0x0004;
/// The i-edge is a mesh (or chunk) boundary.
pub(super) const I_BNDY: u16 = 0x0008;
/// The j-edge is a mesh (or chunk) boundary.
pub(super) const J_BNDY: u16 = 0x0010;
/// The i-edge starts a curve into the zone on its left.
pub(super) const I0_START: u16 = 0x0020;
/// The i-edge starts a curve into the zone on its right.
pub(super) const I1_START: u16 = 0x0040;
/// The j-edge starts a curve into the zone below.
pub(super) const J0_START: u16 = 0x0080;
/// The j-edge starts a curve into the zone above.
pub(super) const J1_START: u16 = 0x0100;
/// The next start is in this row (speeds up the second pass).
pub(super) const START_ROW: u16 = 0x0200;
/// The i-edge begins the upstroke of a slit.
pub(super) const SLIT_UP: u16 = 0x0400;
/// The i-edge begins the downstroke of a slit.
pub(super) const SLIT_DN: u16 = 0x0800;
/// A start of a single-level curve whose other end is on a boundary.
pub(super) const OPEN_END: u16 = 0x1000;
/// The final start point.
pub(super) const ALL_DONE: u16 = 0x2000;
/// The second pass has visited this slit downstroke.
pub(super) const SLIT_DN_VISITED: u16 = 0x4000;
/// Every start mark.
pub(super) const ANY_START: u16 = I0_START | I1_START | J0_START | J1_START;

/// The zones that exist when the points `mask` marks are masked: every
/// zone touching a masked point is left out (gcntr's `mask_zones`).
pub(super) fn mask_zones(imax: usize, jmax: usize, mask: &[bool]) -> Vec<bool> {
    let mut zones = vec![false; imax * jmax + imax + 1];
    zones[imax + 1..imax * jmax].fill(true);
    for j in 0..jmax {
        for i in 0..imax {
            let ij = i + j * imax;
            if i == 0 || j == 0 {
                zones[ij] = false;
            }
            if mask[ij] {
                for zone in [ij, ij + 1, ij + imax, ij + imax + 1] {
                    zones[zone] = false;
                }
            }
        }
    }
    zones
}

impl Mpl2005 {
    /// Classifies the mesh for the levels `zlevel` (a single level when
    /// both are equal) and returns the number of start marks.
    pub(super) fn data_init(&mut self, zlevel: [f64; 2]) -> isize {
        let (imax, jmax) = (self.mesh.imax, self.jmax);
        let ijmax = imax * jmax;
        let two_levels = zlevel[1] > zlevel[0];
        let level = |z: f64| -> u16 {
            if z > zlevel[0] {
                if two_levels && z > zlevel[1] { 2 } else { 1 }
            } else {
                0
            }
        };
        // Lines are not chunked: their start points would not be right.
        let (i_chunk, j_chunk) = if two_levels {
            self.chunk
        } else {
            (imax - 1, jmax - 1)
        };
        let z = &self.mesh.z;
        let data = &mut self.data;
        let at = |data: &[u16], index: isize| data[slot(index)];
        data[0] = level(z[0]);
        let mut count = 0;
        let mut started = false;
        let mut jchunk = 0;
        let mut ij: isize = 0;
        for j in 0..jmax {
            let (mut ichunk, mut i_was_chunk) = (0, false);
            for i in 0..imax {
                // Zone existence one row ahead, for the points and j-edges
                // of this row.
                let exists = match &self.zones {
                    Some(zones) => zones[slot(ij + imax + 1)],
                    None => i < imax - 1 && j < jmax - 1,
                };
                data[slot(ij + imax + 1)] = if exists { ZONE_EX } else { 0 };
                if ij < imax {
                    data[slot(ij + 1)] = 0;
                }
                if ij < ijmax - 1 {
                    data[slot(ij + 1)] |= level(z[slot(ij + 1)]);
                }
                let zone = at(data, ij) & ZONE_EX;
                let ibndy = i == ichunk || zone != at(data, ij + 1) & ZONE_EX;
                let jbndy = j == jchunk || zone != at(data, ij + imax) & ZONE_EX;
                let mut item = at(data, ij);
                if ibndy {
                    item |= I_BNDY;
                }
                if jbndy {
                    item |= J_BNDY;
                }
                let mut marks = 0;
                if j > 0 {
                    marks |= i_edge_starts(data, ij, imax, ibndy);
                }
                if i > 0 && jbndy {
                    marks |= j_edge_starts(data, ij, imax, two_levels, i_was_chunk);
                }
                count += marks.count_ones();
                data[slot(ij)] = item | marks;
                i_was_chunk = i == ichunk;
                if i_was_chunk {
                    ichunk += i_chunk;
                }
                ij += 1;
            }
            if j == jchunk {
                jchunk += j_chunk;
            }
            if count > 0 && !started {
                data[slot(ij - imax)] |= START_ROW;
                started = true;
            }
        }
        if count == 0 {
            data[0] |= ALL_DONE;
        } else {
            self.saddle.fill(0);
        }
        isize::try_from(count).expect("one count per edge")
    }
}

/// The start marks of the i-edge `ij`: only cut edges start curves, and
/// an interior one only if no j-edge sharing its lower point is cut and
/// the filled region lies on the side it starts into.
fn i_edge_starts(data: &[u16], ij: isize, imax: isize, boundary: bool) -> u16 {
    let z = |index: isize| data[slot(index)] & Z_VALUE;
    let exists = |index: isize| data[slot(index)] & ZONE_EX != 0;
    let (v0, vb) = (z(ij), z(ij - imax));
    if v0 == vb {
        return 0;
    }
    let mut marks = 0;
    if boundary {
        if exists(ij) {
            marks |= I0_START;
        }
        if exists(ij + 1) {
            marks |= I1_START;
        }
    } else {
        let (va, vc, vd) = (z(ij - 1), z(ij + 1), z(ij - imax + 1));
        if v0 != 1 && va != v0 && (vc != v0 || vd != v0) && exists(ij) {
            marks |= I0_START;
        }
        if vb != 1 && va == vb && (vc == vb || vd == vb) && exists(ij + 1) {
            marks |= I1_START;
        }
    }
    marks
}

/// The start marks of the boundary j-edge `ij`: every cut one, and for
/// filled contours a few uncut ones where a drawn part of the boundary
/// begins.
fn j_edge_starts(data: &[u16], ij: isize, imax: isize, two_levels: bool, i_was_chunk: bool) -> u16 {
    let z = |index: isize| data[slot(index)] & Z_VALUE;
    let exists = |index: isize| data[slot(index)] & ZONE_EX != 0;
    let (v0, vb) = (z(ij), z(ij - 1));
    let (below, above) = (exists(ij), exists(ij + imax));
    if v0 != vb {
        let mut marks = 0;
        if below {
            marks |= J0_START;
        }
        if above {
            marks |= J1_START;
        }
        marks
    } else if two_levels && v0 == 1 {
        let upper_left = exists(ij + imax - 1);
        if above {
            // The lower left is a drawn part of the boundary.
            if i_was_chunk || !upper_left {
                J1_START
            } else {
                0
            }
        } else if below && upper_left {
            // An open hole at the lower left.
            J0_START
        } else {
            0
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_points_remove_the_zones_around_them() {
        // A 3 × 3 mesh with its centre masked: no zone is left.
        let mut mask = [false; 9];
        mask[4] = true;
        assert!(mask_zones(3, 3, &mask).iter().all(|&exists| !exists));
        let zones = mask_zones(3, 3, &[false; 9]);
        let existing: Vec<usize> = (0..zones.len()).filter(|&ij| zones[ij]).collect();
        assert_eq!(existing, [4, 5, 7, 8]);
    }
}
