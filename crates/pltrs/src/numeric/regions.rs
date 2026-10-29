//! Index regions: the runs of set flags in a mask and the perimeters of
//! the blocks a grid is cut into (matplotlib's `cbook.contiguous_regions`
//! and `cbook._array_patch_perimeters`).

/// The `(start, stop)` index ranges of the runs of set flags, `stop`
/// exclusive (matplotlib's `cbook.contiguous_regions`).
///
/// ```
/// use pltrs::numeric::contiguous_regions;
/// assert_eq!(contiguous_regions(&[true, true, false, true]), [(0, 2), (3, 4)]);
/// assert!(contiguous_regions(&[false; 3]).is_empty());
/// ```
pub fn contiguous_regions(mask: &[bool]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut start = None;
    for (i, &on) in mask.iter().enumerate() {
        match (on, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                regions.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        regions.push((s, mask.len()));
    }
    regions
}

/// Every `stride`-th index below `len - 1`, then `len - 1` (the rows or
/// columns `plot_surface` samples).
fn sampled(len: usize, stride: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }
    let mut indices: Vec<usize> = (0..len - 1).step_by(stride).collect();
    indices.push(len - 1);
    indices
}

/// The `(row, column)` cells on the perimeter of each block of a `rows ×
/// cols` grid cut at every `rstride`-th row and `cstride`-th column and at
/// the last ones, blocks row by row. Neighbouring blocks share their
/// edges; each perimeter starts at the block's top-left cell and runs along
/// the top row, down the right column, back along the bottom row and up
/// the left column, without repeating a corner (matplotlib's
/// `_array_perimeter` of each block, which `_array_patch_perimeters`
/// vectorises when the strides divide the grid). Zero strides give no
/// blocks.
///
/// ```
/// use pltrs::numeric::block_perimeters;
/// let blocks = block_perimeters(3, 3, 2, 2);
/// let cells = [(0, 0), (0, 1), (1, 2), (2, 2), (2, 1), (1, 0)];
/// assert!(cells.iter().all(|cell| blocks[0].contains(cell)));
/// assert_eq!(blocks[0].len(), 8);
/// assert_eq!(block_perimeters(4, 2, 2, 1).len(), 2, "the last row is kept");
/// ```
pub fn block_perimeters(
    rows: usize,
    cols: usize,
    rstride: usize,
    cstride: usize,
) -> Vec<Vec<(usize, usize)>> {
    if rstride == 0 || cstride == 0 {
        return Vec::new();
    }
    let row_cuts = sampled(rows, rstride);
    let col_cuts = sampled(cols, cstride);
    let mut blocks = Vec::new();
    for r in row_cuts.windows(2) {
        for c in col_cuts.windows(2) {
            let (r0, r1, c0, c1) = (r[0], r[1], c[0], c[1]);
            blocks.push(
                (c0..c1)
                    .map(|c| (r0, c))
                    .chain((r0..r1).map(|r| (r, c1)))
                    .chain((c0 + 1..=c1).rev().map(|c| (r1, c)))
                    .chain((r0 + 1..=r1).rev().map(|r| (r, c0)))
                    .collect(),
            );
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_at_either_end() {
        let mask = |runs: &[(bool, usize)]| -> Vec<bool> {
            runs.iter()
                .flat_map(|&(on, n)| std::iter::repeat_n(on, n))
                .collect()
        };
        let (a, b, c, d, e) = (3, 4, 5, 6, 7);
        let expected = [(0, a), (a + b, a + b + c)];
        assert_eq!(
            contiguous_regions(&mask(&[(true, a), (false, b), (true, c)])),
            expected
        );
        assert_eq!(
            contiguous_regions(&mask(&[(true, a), (false, b), (true, c), (false, e)])),
            expected
        );
        let shifted = [(d, d + a), (d + a + b, d + a + b + c)];
        assert_eq!(
            contiguous_regions(&mask(&[(false, d), (true, a), (false, b), (true, c)])),
            shifted
        );
        assert_eq!(contiguous_regions(&[]), Vec::new());
    }

    #[test]
    fn perimeters_trace_each_block() {
        let values = |blocks: Vec<Vec<(usize, usize)>>, cols: usize| -> Vec<Vec<usize>> {
            blocks
                .into_iter()
                .map(|block| block.into_iter().map(|(r, c)| r * cols + c).collect())
                .collect()
        };
        // matplotlib's `_array_perimeter` docstring example.
        assert_eq!(
            values(block_perimeters(3, 4, 2, 3), 10),
            [[0, 1, 2, 3, 13, 23, 22, 21, 20, 10]]
        );
        assert_eq!(
            values(block_perimeters(3, 3, 1, 1), 3),
            [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]]
        );
        assert_eq!(block_perimeters(3, 3, 0, 1), Vec::<Vec<_>>::new());
        assert_eq!(block_perimeters(1, 3, 1, 1), Vec::<Vec<_>>::new());
    }
}
