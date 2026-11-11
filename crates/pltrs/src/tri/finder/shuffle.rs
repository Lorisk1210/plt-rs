//! The order matplotlib's trapezoid map inserts a triangulation's edges
//! in: `std::shuffle` of the edges with a `std::mt19937` seeded with 1234.
//!
//! `std::shuffle` leaves the order to the C++ library, so it depends on
//! the library matplotlib's `_tri` extension was built with: libc++ on
//! macOS (Apple's toolchain), Microsoft's STL on Windows (MSVC 14.51, the
//! linker of matplotlib 3.11.2's `_tri.cp314-win_amd64.pyd`), libstdc++
//! elsewhere (GCC).
//!
//! The Mersenne Twister follows its definition in the C++ standard
//! ([rand.eng.mt]). The libstdc++ order is written from a description of
//! the behaviour, not from its (GPL) source, and checked against it as a
//! black box (the tests hold orders of a program built with g++ 14, the
//! compiler of matplotlib 3.11's Linux wheels):
//!
//! * a uniform integer in `[0, n)` below 2³² comes from Lemire's
//!   multiply-and-reject method ("Fast random integer generation in an
//!   interval", 2019): the high word of `n · draw`, drawn again while the
//!   low word is below `2³² mod n`;
//! * while `n²` fits in the generator's range, two swaps share one draw:
//!   the positions `i` and `i + 1` take `x / (i + 2)` and `x % (i + 2)` of
//!   `x` uniform in `[0, (i + 1)(i + 2))`, after a first lone swap of
//!   position 1 with a coin flip when `n` is even;
//! * otherwise each position `i` from 1 swaps with one uniform in `[0, i]`.
//!
//! The libc++ order, written from a description of its behaviour too
//! and checked the same way (the tests hold orders of a program built with
//! Apple clang 17), walks from the front: position `i` swaps with `i + k`,
//! `k` uniform in `[0, n - 1 - i]`, drawn as the low `w` bits of draws
//! (`w` bits enough for the range), redrawn while out of range.
//!
//! The MSVC order is written from the STL's source (Apache-2.0 with LLVM
//! exception; `_Random_shuffle1` and `_Rng_from_urng_v2` of `<algorithm>`
//! as Lemire's method came in, microsoft/STL#5735, the same on its `main`
//! since): each position `i` from 1 swaps with one uniform in `[0, i]`,
//! drawn by Lemire's method from 32-bit draws (two joined for ranges past
//! 2³²). Below 2³² that is libstdc++'s single loop, without its paired
//! draws. Not yet checked against a program built with MSVC.

/// The 32-bit Mersenne Twister `std::mt19937`.
pub(super) struct Mt19937 {
    state: [u32; Self::N],
    index: usize,
}

impl Mt19937 {
    const N: usize = 624;
    const M: usize = 397;
    const MATRIX: u32 = 0x9908_b0df;
    const UPPER: u32 = 0x8000_0000;

    /// The generator seeded as `std::mt19937 rng(seed)`.
    pub(super) fn new(seed: u32) -> Self {
        let mut state = [0; Self::N];
        state[0] = seed;
        for i in 1..Self::N {
            let previous = state[i - 1];
            // i < 624 fits in u32.
            #[expect(clippy::cast_possible_truncation, reason = "i < 624")]
            let index = i as u32;
            state[i] = 1_812_433_253_u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(index);
        }
        Self {
            state,
            index: Self::N,
        }
    }

    /// The next 32-bit output.
    pub(super) fn next_u32(&mut self) -> u32 {
        if self.index == Self::N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    fn twist(&mut self) {
        for i in 0..Self::N {
            let y = (self.state[i] & Self::UPPER) | (self.state[(i + 1) % Self::N] & !Self::UPPER);
            let mut next = self.state[(i + Self::M) % Self::N] ^ (y >> 1);
            if y & 1 != 0 {
                next ^= Self::MATRIX;
            }
            self.state[i] = next;
        }
        self.index = 0;
    }
}

/// Shuffles `items` as `std::shuffle(items, rng)` does with the C++
/// library of matplotlib's wheels on this platform.
pub(super) fn shuffle<T>(items: &mut [T], rng: &mut Mt19937) {
    if cfg!(target_os = "macos") {
        shuffle_libcxx(items, rng);
    } else if cfg!(target_os = "windows") {
        shuffle_msvc(items, rng);
    } else {
        shuffle_libstdcxx(items, rng);
    }
}

/// The generator's range, `max() - min()`.
const RANGE: u64 = u32::MAX as u64;

/// libstdc++'s order (see the module documentation).
fn shuffle_libstdcxx<T>(items: &mut [T], rng: &mut Mt19937) {
    let n = items.len() as u64;
    if n == 0 {
        return;
    }
    if RANGE / n >= n {
        let mut i = 1;
        if n.is_multiple_of(2) {
            items.swap(i, to_index(uniform_libstdcxx(rng, 1)));
            i += 1;
        }
        while i != items.len() {
            let span = i as u64 + 1;
            let x = uniform_libstdcxx(rng, span * (span + 1) - 1);
            items.swap(i, to_index(x / (span + 1)));
            items.swap(i + 1, to_index(x % (span + 1)));
            i += 2;
        }
    } else {
        for i in 1..items.len() {
            items.swap(i, to_index(uniform_libstdcxx(rng, i as u64)));
        }
    }
}

/// A uniform integer in `[0, high]` as libstdc++'s
/// `uniform_int_distribution` draws it from a 32-bit generator.
fn uniform_libstdcxx(rng: &mut Mt19937, high: u64) -> u64 {
    match u32::try_from(high + 1) {
        Ok(count) => {
            let mut product = u64::from(rng.next_u32()) * u64::from(count);
            // The low word of the product, the remainder Lemire's method
            // tests.
            #[expect(clippy::cast_possible_truncation, reason = "the low word")]
            let mut low = product as u32;
            if low < count {
                let threshold = count.wrapping_neg() % count;
                while low < threshold {
                    product = u64::from(rng.next_u32()) * u64::from(count);
                    #[expect(clippy::cast_possible_truncation, reason = "the low word")]
                    {
                        low = product as u32;
                    }
                }
            }
            product >> 32
        }
        Err(_) if high == RANGE => u64::from(rng.next_u32()),
        Err(_) => loop {
            // Wider than the generator: a high part drawn recursively
            // and a fresh low word, redrawn on overflow.
            let high_part = (RANGE + 1).wrapping_mul(uniform_libstdcxx(rng, high / (RANGE + 1)));
            let value = high_part.wrapping_add(u64::from(rng.next_u32()));
            if value <= high && value >= high_part {
                break value;
            }
        },
    }
}

/// MSVC's order (see the module documentation).
fn shuffle_msvc<T>(items: &mut [T], rng: &mut Mt19937) {
    for i in 1..items.len() {
        let other = to_index(uniform_msvc(rng, i as u64 + 1));
        if other != i {
            items.swap(i, other);
        }
    }
}

/// A uniform integer in `[0, count)` as MSVC's `_Rng_from_urng_v2` draws
/// it for a 64-bit difference type: Lemire's method on one 32-bit draw,
/// or two joined (high word first) when `count - 1` needs more bits.
fn uniform_msvc(rng: &mut Mt19937, count: u64) -> u64 {
    let wide = count - 1 > RANGE;
    let (mask, bits) = if wide { (u64::MAX, 64) } else { (RANGE, 32) };
    let mut draw = || {
        let first = u64::from(rng.next_u32());
        let value = if wide {
            (first << 32) | u64::from(rng.next_u32())
        } else {
            first
        };
        u128::from(value) * u128::from(count)
    };
    let low = |product: u128| {
        #[expect(clippy::cast_possible_truncation, reason = "the low 64 bits")]
        let low = product as u64;
        low & mask
    };
    let mut product = draw();
    if low(product) < count {
        let threshold = mask.wrapping_sub(count).wrapping_add(1) % count;
        while low(product) < threshold {
            product = draw();
        }
    }
    #[expect(clippy::cast_possible_truncation, reason = "below count")]
    let value = (product >> bits) as u64;
    value
}

/// libc++'s order (see the module documentation).
fn shuffle_libcxx<T>(items: &mut [T], rng: &mut Mt19937) {
    let n = items.len();
    if n <= 1 {
        return;
    }
    for first in 0..n - 1 {
        let k = to_index(uniform_libcxx(rng, (n - 1 - first) as u64));
        if k != 0 {
            items.swap(first, first + k);
        }
    }
}

/// A uniform integer in `[0, high]` as libc++'s `uniform_int_distribution`
/// draws it: the `w` low bits of 32-bit draws (`w` the bits of `high + 1`,
/// rounded up), from as many draws as `w` needs, redrawn while above.
fn uniform_libcxx(rng: &mut Mt19937, high: u64) -> u64 {
    if high == 0 {
        return 0;
    }
    let count = high.wrapping_add(1);
    let bits = if count == 0 {
        64
    } else {
        let floor = count.ilog2();
        if count.is_power_of_two() {
            floor
        } else {
            floor + 1
        }
    };
    loop {
        let value = independent_bits(rng, bits);
        if count == 0 || value < count {
            break value;
        }
    }
}

/// libc++'s `__independent_bits_engine` over a generator of exactly 32
/// bits: `bits` bits from `ceil(bits / 32)` draws, the first draws giving
/// `bits / words` bits each and the last `bits % words` one more.
fn independent_bits(rng: &mut Mt19937, bits: u32) -> u64 {
    let words = bits.div_ceil(32);
    let narrow = bits / words;
    let wide_words = bits % words;
    let mask = |width: u32| {
        if width >= 32 {
            u64::from(u32::MAX)
        } else {
            (1_u64 << width) - 1
        }
    };
    let mut value = 0_u64;
    for word in 0..words {
        let width = if word < words - wide_words {
            narrow
        } else {
            narrow + 1
        };
        let shifted = if width >= 64 { 0 } else { value << width };
        value = shifted + (u64::from(rng.next_u32()) & mask(width));
    }
    value
}

/// A position drawn within a slice's length.
fn to_index(position: u64) -> usize {
    usize::try_from(position).expect("a position within the slice")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_matches_the_standards_check_value() {
        // [rand.predef]: the 10000th output of a default-constructed
        // mt19937 (seed 5489) is 4123659995.
        let mut rng = Mt19937::new(5489);
        let output = (0..10_000).map(|_| rng.next_u32()).last();
        assert_eq!(output, Some(4_123_659_995));
    }

    /// An order hash and the first items of `0..n` shuffled by `shuffle`
    /// with the generator seeded with 1234.
    fn shuffled(n: usize, shuffle: fn(&mut [usize], &mut Mt19937)) -> (u64, Vec<usize>) {
        let mut items: Vec<usize> = (0..n).collect();
        shuffle(&mut items, &mut Mt19937::new(1234));
        let hash = items.iter().fold(0_u64, |hash, &item| {
            hash.wrapping_mul(1_000_003).wrapping_add(item as u64)
        });
        (hash, items.into_iter().take(12).collect())
    }

    /// `std::shuffle` of `0..n` with `std::mt19937 rng(1234)`, from a
    /// program built with g++ 14.
    fn libstdcxx(n: usize) -> (u64, Vec<usize>) {
        shuffled(n, shuffle_libstdcxx)
    }

    /// The same from a program built with Apple clang 17 on macOS 26
    /// (libc++).
    fn libcxx(n: usize) -> (u64, Vec<usize>) {
        shuffled(n, shuffle_libcxx)
    }

    #[test]
    fn the_libstdcxx_order_is_g_plus_pluss() {
        assert_eq!(libstdcxx(0), (0, vec![]));
        assert_eq!(libstdcxx(1), (0, vec![0]));
        assert_eq!(libstdcxx(2), (1_000_003, vec![1, 0]));
        assert_eq!(libstdcxx(3), (1_000_008_000_015, vec![1, 2, 0]));
        assert_eq!(libstdcxx(4).1, vec![1, 3, 0, 2]);
        assert_eq!(libstdcxx(5).1, vec![1, 3, 0, 2, 4]);
        assert_eq!(libstdcxx(10).1, vec![5, 3, 0, 8, 2, 7, 1, 6, 4, 9]);
        assert_eq!(
            libstdcxx(17),
            (
                13_328_631_214_048_866_328,
                vec![1, 3, 0, 5, 12, 16, 7, 11, 4, 8, 13, 2]
            )
        );
        assert_eq!(libstdcxx(1000).0, 9_168_228_858_930_938_070);
        // The last size of paired draws and the first of single ones.
        assert_eq!(libstdcxx(65_535).0, 5_442_322_870_142_879_723);
        assert_eq!(libstdcxx(65_536).0, 1_078_152_657_664_495_442);
        assert_eq!(libstdcxx(200_000).0, 14_593_499_966_093_821_954);
    }

    /// MSVC's order of `0..10` (a Python model of the STL's code gives
    /// the same, and g++'s orders where the two agree).
    const MSVC_10: [usize; 10] = [1, 2, 5, 0, 6, 3, 8, 9, 7, 4];
    /// The order hash of MSVC's shuffle of `0..1000`.
    const MSVC_1000: u64 = 16_079_519_895_217_274_278;

    /// The order hash and first items of MSVC's shuffle of `0..n`, as
    /// [`libstdcxx`] gives libstdc++'s.
    fn msvc(n: usize) -> (u64, Vec<usize>) {
        shuffled(n, shuffle_msvc)
    }

    #[test]
    fn the_msvc_order_is_libstdcxxs_single_loop() {
        // Where libstdc++ swaps one position per draw, the orders agree
        // (g++'s orders above).
        assert_eq!(msvc(65_536), libstdcxx(65_536));
        assert_eq!(msvc(200_000), libstdcxx(200_000));
        // The orders a program built with MSVC 14.51 should print (see
        // `hostile-round9` in the notes: not yet run on Windows).
        assert_eq!(msvc(2), (1_000_003, vec![1, 0]));
        assert_eq!(msvc(10).1, MSVC_10);
        assert_eq!(msvc(1000).0, MSVC_1000);
    }

    #[test]
    fn msvc_draws_cover_wide_ranges() {
        let mut rng = Mt19937::new(1);
        assert!((0..200).all(|_| uniform_msvc(&mut rng, (1 << 40) + 3) < (1 << 40) + 3));
        assert!((0..200).all(|_| uniform_msvc(&mut rng, 7) < 7));
        assert_eq!(uniform_msvc(&mut rng, 1), 0);
    }

    #[test]
    fn the_libcxx_order_is_apple_clangs() {
        assert_eq!(libcxx(0), (0, vec![]));
        assert_eq!(libcxx(1), (0, vec![0]));
        assert_eq!(libcxx(2), (1_000_003, vec![1, 0]));
        assert_eq!(libcxx(3), (2_000_012_000_019, vec![2, 0, 1]));
        assert_eq!(libcxx(4).1, vec![3, 0, 1, 2]);
        assert_eq!(libcxx(5).1, vec![3, 0, 1, 2, 4]);
        assert_eq!(libcxx(10).1, vec![3, 7, 1, 2, 8, 5, 0, 4, 9, 6]);
        assert_eq!(
            libcxx(17),
            (
                14_943_143_090_613_215_780,
                vec![15, 4, 8, 2, 16, 9, 14, 1, 5, 7, 11, 13]
            )
        );
        assert_eq!(libcxx(1000).0, 3_464_287_770_629_461_700);
        // Either side of a power of two, where a draw gains a bit.
        assert_eq!(libcxx(65_535).0, 4_848_170_295_527_812_755);
        assert_eq!(libcxx(65_536).0, 9_828_220_514_130_117_512);
        assert_eq!(libcxx(200_000).0, 15_861_224_853_990_749_804);
    }

    #[test]
    fn libcxx_bits_cover_the_range() {
        let mut rng = Mt19937::new(1);
        assert!((0..200).all(|_| uniform_libcxx(&mut rng, 5) <= 5));
        assert!((0..200).all(|_| uniform_libcxx(&mut rng, (1 << 40) + 3) <= (1 << 40) + 3));
        assert_eq!(uniform_libcxx(&mut rng, 0), 0);
    }
}
