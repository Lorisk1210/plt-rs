//! The gamma function as `CPython`'s `math.gamma` computes it, for
//! `fillunder3d`'s Poisson probabilities.

/// The Lanczos parameter `g` of `CPython`'s approximation (exactly
/// 6.024680040776729583740234375).
const LANCZOS_G: f64 = 6.024_680_040_776_73;

/// `LANCZOS_G - 0.5`.
const LANCZOS_G_MINUS_HALF: f64 = 5.524_680_040_776_73;

/// The numerator coefficients of the Lanczos sum, rounded to `f64` as
/// `CPython` stores them.
const NUMERATOR: [f64; 13] = [
    23_531_376_880.410_76,
    42_919_803_642.649_1,
    35_711_959_237.355_67,
    17_921_034_426.037_21,
    6_039_542_586.352_028,
    1_439_720_407.311_721_6,
    248_874_557.862_054_17,
    31_426_415.585_400_194,
    2_876_370.628_935_372_5,
    186_056.265_395_223_48,
    8_071.672_002_365_816,
    210.824_277_751_579_36,
    2.506_628_274_631_000_2,
];

/// The denominator coefficients: those of `x (x + 1) ... (x + 11)`.
const DENOMINATOR: [f64; 13] = [
    0.0,
    39_916_800.0,
    120_543_840.0,
    150_917_976.0,
    105_258_076.0,
    45_995_730.0,
    13_339_535.0,
    2_637_558.0,
    357_423.0,
    32_670.0,
    1_925.0,
    66.0,
    1.0,
];

/// The rational function of the Lanczos approximation at `x > 0`, in
/// powers of `1 / x` from 5 on to avoid overflow (`CPython`'s
/// `lanczos_sum`).
fn lanczos_sum(x: f64) -> f64 {
    let (num, den) = if x < 5.0 {
        NUMERATOR
            .iter()
            .zip(&DENOMINATOR)
            .rev()
            .fold((0.0, 0.0), |(num, den), (n, d)| (num * x + n, den * x + d))
    } else {
        NUMERATOR
            .iter()
            .zip(&DENOMINATOR)
            .fold((0.0, 0.0), |(num, den), (n, d)| (num / x + n, den / x + d))
    };
    num / den
}

/// `Γ(x)` for `0 < x < 140` as `CPython`'s `math.gamma` computes it:
/// factorials for whole numbers up to 23, otherwise the Lanczos
/// approximation with the rounding error of `x + g - 0.5` corrected.
pub(super) fn gamma(x: f64) -> f64 {
    if x.fract() == 0.0 && x <= 23.0 {
        return (2..x as u32).map(f64::from).product();
    }
    let y = x + LANCZOS_G_MINUS_HALF;
    let error = if x > LANCZOS_G_MINUS_HALF {
        (y - x) - LANCZOS_G_MINUS_HALF
    } else {
        (y - LANCZOS_G_MINUS_HALF) - x
    };
    let z = error * LANCZOS_G / y;
    let r = lanczos_sum(x) / y.exp();
    (r + z * r) * y.powf(x - 0.5)
}

#[cfg(test)]
mod tests {
    use super::gamma;

    #[test]
    fn whole_numbers_are_factorials() {
        assert_eq!(gamma(1.0), 1.0);
        assert_eq!(gamma(5.0), 24.0);
        assert_eq!(gamma(11.0), 3_628_800.0);
    }

    #[test]
    fn fractions_match_python() {
        // math.gamma(0.5), math.gamma(4.5), math.gamma(10 / 3 + 1)
        assert_eq!(gamma(0.5), 1.772_453_850_905_515_9);
        assert_eq!(gamma(4.5), 11.631_728_396_567_446);
        assert_eq!(gamma(10.0 / 3.0 + 1.0), 9.260_528_268_125_555);
    }
}
