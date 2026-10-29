//! Coverage-to-alpha transfer functions (Agg's gamma functions).

/// Maps geometric coverage to alpha before compositing.
///
/// The rasterizer tabulates the function once (`uround(f(i/255)·255)` for
/// each 8-bit coverage, as Agg's `rasterizer_scanline_aa::gamma` does).
/// matplotlib never sets a gamma, so [`Gamma::Linear`] is the faithful
/// default; note that matplotlib's `antialiased=False` is *not* a threshold
/// but the binary renderer (see [`Antialias::Off`](crate::Antialias::Off)).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Gamma {
    /// Alpha equals coverage (Agg's `gamma_none`).
    #[default]
    Linear,
    /// Alpha is 0 below the threshold coverage (in `[0, 1]`) and 1 at or
    /// above it (Agg's `gamma_threshold`).
    Threshold(f64),
    /// Alpha is `coverage^γ` (Agg's `gamma_power`).
    Power(f64),
}

impl Gamma {
    /// The 256-entry lookup table for this function.
    pub(crate) fn table(self) -> [u8; 256] {
        let mut lut = [0u8; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            let x = i as f64 / 255.0;
            let y = match self {
                Self::Linear => {
                    *v = i as u8;
                    continue;
                }
                Self::Threshold(t) => {
                    if x < t {
                        0.0
                    } else {
                        1.0
                    }
                }
                Self::Power(g) => x.powf(g),
            };
            *v = (y * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
        lut
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables() {
        let linear = Gamma::Linear.table();
        assert!(linear.iter().enumerate().all(|(i, &v)| v as usize == i));
        let t = Gamma::Threshold(0.5).table();
        assert_eq!((t[127], t[128]), (0, 255));
        let p = Gamma::Power(2.0).table();
        assert_eq!((p[0], p[255], p[128]), (0, 255, 64));
    }
}
