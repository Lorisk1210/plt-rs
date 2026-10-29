//! Backward references: LZ77 over pixels with hash chains, preferring the
//! pixel to the left and the one above, whose two-dimensional distance
//! codes are the shortest.

use crate::webp::lossless::format::CODE_TO_PLANE;

/// The longest copy a length code can express.
const MAX_LENGTH: usize = 4096;
/// The farthest a distance code can reach (codes past 120 are plain
/// distances plus 120, up to `1 << 20`).
const WINDOW: usize = (1 << 20) - 120;
/// The shortest copy worth coding.
const MIN_LENGTH: usize = 3;
/// How many earlier positions with the same hash are tried.
const MAX_CANDIDATES: usize = 48;
/// The bits of the hash of two pixels.
const HASH_BITS: u32 = 16;

/// For each two-dimensional offset `(rows up) << 4 | (8 - columns left)`,
/// its index in [`CODE_TO_PLANE`].
const PLANE_INDEX: [u8; 128] = {
    let mut index = [0u8; 128];
    let mut code = 0;
    while code < CODE_TO_PLANE.len() {
        index[CODE_TO_PLANE[code] as usize] = code as u8;
        code += 1;
    }
    index
};

/// A step of the coded pixel stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Token {
    /// A pixel coded by its channels (or by the colour cache).
    Literal(u32),
    /// `length` pixels copied from the one the distance code names.
    Copy { length: usize, distance_code: usize },
}

impl Token {
    /// How many pixels the token stands for.
    pub(super) fn len(self) -> usize {
        match self {
            Self::Literal(_) => 1,
            Self::Copy { length, .. } => length,
        }
    }
}

/// The distance code of `distance` in rows of `width` pixels: short
/// offsets up to seven rows up have codes 1–120 (libwebp's
/// `DistanceToPlaneCode`), the rest `distance + 120`.
pub(super) fn distance_code(width: usize, distance: usize) -> usize {
    let rows = distance / width;
    let columns = distance - rows * width;
    if columns <= 8 && rows < 8 {
        usize::from(PLANE_INDEX[rows * 16 + 8 - columns]) + 1
    } else if columns + 8 > width && rows < 7 {
        usize::from(PLANE_INDEX[(rows + 1) * 16 + 8 + (width - columns)]) + 1
    } else {
        distance + 120
    }
}

/// The hash of the two pixels from `position`.
fn hash(pixels: &[u32], position: usize) -> usize {
    let pair = (u64::from(pixels[position]) << 32) | u64::from(pixels[position + 1]);
    (pair.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> (64 - HASH_BITS)) as usize
}

/// How many pixels from `position` repeat those `distance` earlier.
fn match_length(pixels: &[u32], position: usize, distance: usize) -> usize {
    let limit = MAX_LENGTH.min(pixels.len() - position);
    pixels[position..position + limit]
        .iter()
        .zip(&pixels[position - distance..])
        .take_while(|(a, b)| a == b)
        .count()
}

/// Remembers the positions of each hash, most recent first.
#[derive(Debug)]
struct Chains {
    head: Vec<usize>,
    previous: Vec<usize>,
}

impl Chains {
    /// No position stored.
    const NONE: usize = usize::MAX;

    fn new(len: usize) -> Self {
        Self {
            head: vec![Self::NONE; 1 << HASH_BITS],
            previous: vec![Self::NONE; len],
        }
    }

    fn insert(&mut self, pixels: &[u32], position: usize) {
        if position + 1 < pixels.len() {
            let key = hash(pixels, position);
            self.previous[position] = self.head[key];
            self.head[key] = position;
        }
    }

    /// The earlier positions sharing the hash at `position`, nearest
    /// first, within the window.
    fn candidates<'a>(
        &'a self,
        pixels: &[u32],
        position: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        let start = if position + 1 < pixels.len() {
            self.head[hash(pixels, position)]
        } else {
            Self::NONE
        };
        std::iter::successors((start != Self::NONE).then_some(start), |&candidate| {
            let next = self.previous[candidate];
            (next != Self::NONE).then_some(next)
        })
        .take_while(move |&candidate| position - candidate <= WINDOW)
        .take(MAX_CANDIDATES)
    }
}

/// Splits rows of `width` pixels into literals and copies, greedily
/// taking the longest copy: from the left neighbour, from the pixel above,
/// or from an earlier position of the same hash.
pub(super) fn tokens(pixels: &[u32], width: usize) -> Vec<Token> {
    let mut chains = Chains::new(pixels.len());
    let mut tokens = Vec::new();
    let mut position = 0;
    while position < pixels.len() {
        let neighbours = [1, width]
            .into_iter()
            .filter(|&distance| distance <= position);
        let best = neighbours
            .chain(
                chains
                    .candidates(pixels, position)
                    .map(|candidate| position - candidate),
            )
            .map(|distance| (match_length(pixels, position, distance), distance))
            .fold(
                (0, 0),
                |best, found| if found.0 > best.0 { found } else { best },
            );
        let (length, distance) = best;
        let token = if length >= MIN_LENGTH {
            Token::Copy {
                length,
                distance_code: distance_code(width, distance),
            }
        } else {
            Token::Literal(pixels[position])
        };
        for covered in position..position + token.len() {
            chains.insert(pixels, covered);
        }
        position += token.len();
        tokens.push(token);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distance a code stands for, as the decoder computes it.
    fn decoded_distance(width: usize, code: usize) -> usize {
        if code > 120 {
            return code - 120;
        }
        let offset = CODE_TO_PLANE[code - 1];
        let distance = i64::from(offset >> 4) * width as i64 + 8 - i64::from(offset & 0xF);
        usize::try_from(distance).unwrap().max(1)
    }

    #[test]
    fn distance_codes_round_trip() {
        for width in [1, 2, 3, 7, 8, 9, 16, 100] {
            for distance in 1..2000 {
                let code = distance_code(width, distance);
                assert_eq!(
                    decoded_distance(width, code),
                    distance,
                    "width {width}, distance {distance}"
                );
            }
        }
        assert_eq!(distance_code(100, 100), 1, "straight up is the first code");
        assert_eq!(distance_code(100, 1), 2, "the left neighbour is the second");
    }

    #[test]
    fn runs_become_copies_of_the_left_neighbour() {
        let pixels = [5, 5, 5, 5, 5, 7, 8, 9, 7, 8, 9];
        let found = tokens(&pixels, 11);
        assert_eq!(
            found,
            [
                Token::Literal(5),
                Token::Copy {
                    length: 4,
                    distance_code: 2
                },
                Token::Literal(7),
                Token::Literal(8),
                Token::Literal(9),
                Token::Copy {
                    length: 3,
                    distance_code: distance_code(11, 3)
                },
            ]
        );
        let rows = [1, 2, 3, 4, 1, 2, 3, 4];
        assert_eq!(
            tokens(&rows, 4)[4],
            Token::Copy {
                length: 4,
                distance_code: 1
            },
            "the row above"
        );
    }
}
