//! RGB ↔ HSV conversion (matplotlib's `rgb_to_hsv` / `hsv_to_rgb`).

#![allow(
    clippy::float_cmp,
    reason = "matplotlib's hue selection compares channels with the maximum exactly"
)]

use plt_core::Rgba;

use super::ColorError;
use super::npmath::{nan_max, nan_min};
use crate::ticker::pymath::py_mod;

/// Converts `[r, g, b]` in `[0, 1]` to `[hue, saturation, value]`, all in
/// `[0, 1]`. NaN components pass through like in matplotlib.
///
/// ```
/// assert_eq!(plt::colors::rgb_to_hsv([1.0, 0.0, 0.0])?, [0.0, 1.0, 1.0]);
/// # Ok::<(), plt::colors::ColorError>(())
/// ```
pub fn rgb_to_hsv(rgb: [f64; 3]) -> Result<[f64; 3], ColorError> {
    let [r, g, b] = rgb;
    let max = nan_max(nan_max(r, g), b);
    let min = nan_min(nan_min(r, g), b);
    if max > 1.0 || min < 0.0 {
        return Err(ColorError::ComponentOutOfRange(Rgba::opaque(r, g, b)));
    }
    let delta = max - min;
    let saturation = if max > 0.0 { delta / max } else { 0.0 };
    // Later matches win, as in matplotlib's masked assignments.
    let mut hue = 0.0;
    if delta > 0.0 {
        if r == max {
            hue = (g - b) / delta;
        }
        if g == max {
            hue = 2.0 + (b - r) / delta;
        }
        if b == max {
            hue = 4.0 + (r - g) / delta;
        }
    }
    Ok([py_mod(hue / 6.0, 1.0), saturation, max])
}

/// Converts `[hue, saturation, value]` to `[r, g, b]`.
///
/// ```
/// assert_eq!(plt::colors::hsv_to_rgb([0.5, 1.0, 1.0]), [0.0, 1.0, 1.0]);
/// ```
pub fn hsv_to_rgb(hsv: [f64; 3]) -> [f64; 3] {
    let [h, s, v] = hsv;
    if s == 0.0 {
        return [v; 3];
    }
    let sector = (h * 6.0) as i64;
    let f = h * 6.0 - sector as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match sector.rem_euclid(6) {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_and_greys() {
        assert_eq!(rgb_to_hsv([0.0, 1.0, 0.0]), Ok([1.0 / 3.0, 1.0, 1.0]));
        assert_eq!(rgb_to_hsv([0.0, 0.0, 1.0]), Ok([2.0 / 3.0, 1.0, 1.0]));
        assert_eq!(rgb_to_hsv([0.5, 0.5, 0.5]), Ok([0.0, 0.0, 0.5]));
        assert_eq!(rgb_to_hsv([0.0, 0.0, 0.0]), Ok([0.0, 0.0, 0.0]));
        assert_eq!(rgb_to_hsv([1.0, 0.0, 0.5]).map(|h| h[0]), Ok(11.0 / 12.0));
        assert!(rgb_to_hsv([1.5, 0.0, 0.0]).is_err());
        assert!(rgb_to_hsv([-0.5, 0.0, 0.0]).is_err());
    }

    #[test]
    fn inverse_conversion() {
        assert_eq!(hsv_to_rgb([0.0, 1.0, 1.0]), [1.0, 0.0, 0.0]);
        assert_eq!(hsv_to_rgb([1.0, 1.0, 1.0]), [1.0, 0.0, 0.0], "hue 1 wraps");
        assert_eq!(hsv_to_rgb([0.3, 0.0, 0.7]), [0.7; 3]);
        assert_eq!(hsv_to_rgb([0.0, 0.25, 0.75]), [0.75, 0.5625, 0.5625]);
        for rgb in [[0.2, 0.4, 0.6], [0.9, 0.1, 0.1], [0.25, 0.75, 0.75]] {
            let back = hsv_to_rgb(rgb_to_hsv(rgb).unwrap());
            assert!(back.iter().zip(rgb).all(|(a, b)| (a - b).abs() < 1e-12));
        }
    }
}
