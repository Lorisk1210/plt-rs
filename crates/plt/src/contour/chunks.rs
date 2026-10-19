//! How a quad grid is split into chunks contoured one after another
//! (contourpy's chunk sizes and limits, shared by its algorithms).

/// How the grid is split into independently contoured chunks.
#[derive(Clone, Copy, Debug)]
pub(super) struct Chunks {
    /// Quads per chunk in x (at least 1).
    pub(super) x_size: usize,
    /// Quads per chunk in y (at least 1).
    pub(super) y_size: usize,
    /// Number of chunks in x.
    pub(super) nx: usize,
    /// Number of chunks in y.
    pub(super) ny: usize,
    /// Grid points in x and y.
    pub(super) points: (usize, usize),
}

impl Chunks {
    /// Chunks of `(y, x)` quads (0 = the whole grid) of an `nx × ny`
    /// point grid (contourpy's `calc_chunk_size`/`calc_chunk_count`).
    pub(super) fn new(nx: usize, ny: usize, (y_size, x_size): (usize, usize)) -> Self {
        let size = |points: usize, requested: usize| {
            let quads = points - 1;
            if requested > 0 {
                requested.min(quads)
            } else {
                quads
            }
            .max(1)
        };
        let count = |points: usize, size: usize| (points - 1).div_ceil(size);
        let (x_size, y_size) = (size(nx, x_size), size(ny, y_size));
        Self {
            x_size,
            y_size,
            nx: count(nx, x_size),
            ny: count(ny, y_size),
            points: (nx, ny),
        }
    }

    /// The chunks in sweep order (contourpy's `get_chunk_limits`).
    pub(super) fn limits(self) -> impl Iterator<Item = ChunkLimits> {
        (0..self.nx * self.ny).map(move |ij| {
            let (ichunk, jchunk) = (ij % self.nx, ij / self.nx);
            let end = |chunk: usize, count: usize, size: usize, points: usize| {
                if chunk == count - 1 {
                    points
                } else {
                    (chunk + 1) * size
                }
            };
            ChunkLimits {
                ichunk,
                jchunk,
                istart: ichunk * self.x_size,
                iend: end(ichunk, self.nx, self.x_size, self.points.0),
                jstart: jchunk * self.y_size,
                jend: end(jchunk, self.ny, self.y_size, self.points.1),
            }
        })
    }
}

/// The index range of one chunk (points `istart..iend` × `jstart..jend`;
/// the last chunk extends to the grid edge).
#[derive(Clone, Copy, Debug)]
pub(super) struct ChunkLimits {
    pub(super) ichunk: usize,
    pub(super) jchunk: usize,
    pub(super) istart: usize,
    pub(super) iend: usize,
    pub(super) jstart: usize,
    pub(super) jend: usize,
}

impl ChunkLimits {
    /// The quads of the chunk, row by row.
    pub(super) fn quads(self, nx: usize) -> impl Iterator<Item = usize> {
        (self.jstart..self.jend).flat_map(move |j| (self.istart + j * nx)..(self.iend + j * nx))
    }
}
