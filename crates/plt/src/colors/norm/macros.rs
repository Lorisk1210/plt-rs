//! Boilerplate shared by norms that store their limits in a `Limits`.

/// `with_vmin`/`with_vmax`/`with_clip` builders for norms whose state
/// includes a `limits: Limits` field.
macro_rules! limit_builders {
    (@methods) => {
            /// The same norm with the data value mapped to 0.
            #[must_use]
            pub fn with_vmin(mut self, vmin: f64) -> Self {
                self.limits.vmin = Some(vmin);
                self
            }

            /// The same norm with the data value mapped to 1.
            #[must_use]
            pub fn with_vmax(mut self, vmax: f64) -> Self {
                self.limits.vmax = Some(vmax);
                self
            }

            /// The same norm, clipping out-of-range values to the limits
            /// when `clip` is set.
            #[must_use]
            pub fn with_clip(mut self, clip: bool) -> Self {
                self.limits.clip = clip;
                self
            }
    };
    (for<$param:ident: $bound:path> $ty:ty) => {
        impl<$param: $bound> $ty {
            limit_builders!(@methods);
        }
    };
    ($ty:ty) => {
        impl $ty {
            limit_builders!(@methods);
        }
    };
}

/// The [`Norm`] limit accessors for norms with a `limits: Limits` field.
macro_rules! limit_accessors {
    () => {
        fn vmin(&self) -> Option<f64> {
            self.limits.vmin
        }

        fn vmax(&self) -> Option<f64> {
            self.limits.vmax
        }

        fn set_vmin(&mut self, vmin: Option<f64>) {
            self.limits.vmin = vmin;
        }

        fn set_vmax(&mut self, vmax: Option<f64>) {
            self.limits.vmax = vmax;
        }

        fn clip(&self) -> bool {
            self.limits.clip
        }

        fn set_clip(&mut self, clip: bool) {
            self.limits.clip = clip;
        }
    };
}
