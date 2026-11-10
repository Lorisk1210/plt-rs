//! The one-dimensional maps behind the non-linear scales.
//!
//! Each map implements [`AxisMap`] so axes can plug it straight into their
//! data transform. The formulas follow matplotlib's transform classes
//! operation by operation, so results agree to the last bit wherever the
//! underlying `libm` functions do.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

use pltrs_core::archive::{External, ProgramFn};
use pltrs_core::numeric::{np_arcsinh, np_sign};

use super::{NonPositive, ScaleError};
use crate::numeric::numpy_arith::warn_elementwise;
use crate::ticker::LogBase;
use crate::transforms::AxisMap;

/// The value non-positive inputs map to when clipping (`log10` of the
/// smallest double is about -323, so -1000 is safely beyond).
const CLIPPED: f64 = 1000.0;

/// `log_base(x)` (matplotlib's `LogTransform`), with non-positive inputs
/// clipped to -1000 or mapped to NaN/-inf.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogTransform {
    base: LogBase,
    nonpositive: NonPositive,
}

pltrs_core::archive_struct!(LogTransform { base, nonpositive });

impl LogTransform {
    /// A log map; `base` must be positive and not 1.
    pub fn new(base: f64, nonpositive: NonPositive) -> Result<Self, ScaleError> {
        let base = LogBase::new(base)
            .ok_or_else(|| ScaleError::invalid("base", "the log base cannot be <= 0 or == 1"))?;
        Ok(Self::from_base(base, nonpositive))
    }

    /// A log map in `base`.
    pub fn from_base(base: LogBase, nonpositive: NonPositive) -> Self {
        Self { base, nonpositive }
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base.get()
    }

    /// The logarithm base, as checked on construction.
    pub fn log_base(&self) -> LogBase {
        self.base
    }

    /// The treatment of non-positive values.
    pub fn nonpositive(&self) -> NonPositive {
        self.nonpositive
    }
}

impl AxisMap for LogTransform {
    fn forward(&self, value: f64) -> f64 {
        let base = self.base();
        let out = if base == std::f64::consts::E {
            value.ln()
        } else if base == 2.0 {
            value.log2()
        } else if base == 10.0 {
            value.log10()
        } else {
            value.ln() / base.ln()
        };
        if self.nonpositive == NonPositive::Clip && value <= 0.0 {
            -CLIPPED
        } else {
            out
        }
    }

    /// Non-positive values are masked whatever `nonpositive` says.
    fn forward_masked(&self, value: f64) -> f64 {
        if value > 0.0 {
            self.forward(value)
        } else {
            f64::NAN
        }
    }

    fn inverse(&self, value: f64) -> f64 {
        let base = self.base();
        if base == std::f64::consts::E {
            value.exp()
        } else if base == 2.0 {
            value.exp2()
        } else {
            (value * base.ln()).exp()
        }
    }

    /// `InvertedLogTransform`: `np.exp`, `np.exp2`, or `np.exp(values *
    /// np.log(base))` for other bases.
    fn warn_inverting(&self, values: &[f64]) {
        let base = self.base();
        if base == std::f64::consts::E {
            warn_elementwise("exp", values.iter().map(|&v| (v.exp(), v, v)));
        } else if base == 2.0 {
            warn_elementwise("exp2", values.iter().map(|&v| (v.exp2(), v, v)));
        } else {
            let ln = base.ln();
            let scaled: Vec<f64> = values.iter().map(|&v| v * ln).collect();
            warn_elementwise(
                "multiply",
                values.iter().zip(&scaled).map(|(&v, &s)| (s, v, ln)),
            );
            warn_elementwise("exp", scaled.iter().map(|&s| (s.exp(), s, s)));
        }
    }
}

/// Linear within `±linthresh`, logarithmic beyond (matplotlib's
/// `SymmetricalLogTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymmetricalLogTransform {
    base: f64,
    linthresh: f64,
    linscale: f64,
}

pltrs_core::archive_struct!(SymmetricalLogTransform {
    base,
    linthresh,
    linscale
});

impl SymmetricalLogTransform {
    /// A symlog map; `base > 1`, `linthresh > 0` and `linscale > 0`.
    pub fn new(base: f64, linthresh: f64, linscale: f64) -> Result<Self, ScaleError> {
        if base.is_nan() || base <= 1.0 {
            return Err(ScaleError::invalid("base", "must be larger than 1"));
        }
        if linthresh.is_nan() || linthresh <= 0.0 {
            return Err(ScaleError::invalid("linthresh", "must be positive"));
        }
        if linscale.is_nan() || linscale <= 0.0 {
            return Err(ScaleError::invalid("linscale", "must be positive"));
        }
        Ok(Self {
            base,
            linthresh,
            linscale,
        })
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The extent of the linear region around zero.
    pub fn linthresh(&self) -> f64 {
        self.linthresh
    }

    /// The number of decades each half of the linear region spans.
    pub fn linscale(&self) -> f64 {
        self.linscale
    }

    fn linscale_adj(&self) -> f64 {
        self.linscale / (1.0 - 1.0 / self.base)
    }
}

impl AxisMap for SymmetricalLogTransform {
    fn forward(&self, value: f64) -> f64 {
        let adj = self.linscale_adj();
        if value.abs() <= self.linthresh {
            return value * adj;
        }
        let log_base = self.base.ln();
        np_sign(value)
            * self.linthresh
            * (adj - self.linthresh.ln() / log_base + value.abs().ln() / log_base)
    }

    fn inverse(&self, value: f64) -> f64 {
        let adj = self.linscale_adj();
        if value.abs() <= self.forward(self.linthresh) {
            return value / adj;
        }
        np_sign(value)
            * self.linthresh
            * ((value.abs() / self.linthresh - adj) * self.base.ln()).exp()
    }
}

/// `log10(p / (1 - p))` (matplotlib's `LogitTransform`); clipping maps
/// `p <= 0` to -1000 and `p >= 1` to 1000.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogitTransform {
    nonpositive: NonPositive,
}

pltrs_core::archive_struct!(LogitTransform { nonpositive });

impl LogitTransform {
    /// A logit map.
    pub fn new(nonpositive: NonPositive) -> Self {
        Self { nonpositive }
    }

    /// The treatment of values outside `(0, 1)`.
    pub fn nonpositive(&self) -> NonPositive {
        self.nonpositive
    }
}

impl AxisMap for LogitTransform {
    fn forward(&self, value: f64) -> f64 {
        let out = (value / (1.0 - value)).log10();
        match self.nonpositive {
            NonPositive::Clip if value <= 0.0 => -CLIPPED,
            NonPositive::Clip if value >= 1.0 => CLIPPED,
            _ => out,
        }
    }

    /// Values outside `(0, 1)` are masked whatever `nonpositive` says.
    fn forward_masked(&self, value: f64) -> f64 {
        if 0.0 < value && value < 1.0 {
            self.forward(value)
        } else {
            f64::NAN
        }
    }

    fn inverse(&self, value: f64) -> f64 {
        1.0 / (1.0 + pltrs_core::numeric::pow(10.0, -value))
    }

    /// `LogisticTransform`: `1.0 / (1 + 10**(-values))`, whose power
    /// overflows for large negative values.
    fn warn_inverting(&self, values: &[f64]) {
        warn_elementwise(
            "power",
            values
                .iter()
                .map(|&v| (pltrs_core::numeric::pow(10.0, -v), 10.0, -v)),
        );
    }
}

/// `w * asinh(x / w)` (matplotlib's `AsinhTransform`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AsinhTransform {
    linear_width: f64,
}

pltrs_core::archive_struct!(AsinhTransform { linear_width });

impl AsinhTransform {
    /// An asinh map; `linear_width` must be positive.
    pub fn new(linear_width: f64) -> Result<Self, ScaleError> {
        if linear_width > 0.0 {
            Ok(Self { linear_width })
        } else {
            Err(ScaleError::invalid(
                "linear_width",
                "must be strictly positive",
            ))
        }
    }

    /// The width of the quasi-linear region.
    pub fn linear_width(&self) -> f64 {
        self.linear_width
    }
}

impl AxisMap for AsinhTransform {
    fn forward(&self, value: f64) -> f64 {
        self.linear_width * np_arcsinh(value / self.linear_width)
    }

    fn inverse(&self, value: f64) -> f64 {
        self.linear_width * (value / self.linear_width).sinh()
    }
}

/// How the values a function of the program's maps are laid out in the
/// array it is handed, as matplotlib hands them: drawing gives a
/// one-dimensional transform a row of values (`transform_non_affine` of a
/// slice of the points), `Transform.transform` a column (`(N, 1)`), and
/// a mesh's drawing a masked row (its coordinates are masked arrays).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrayLayout {
    /// A one-dimensional array, `(N,)`.
    Row,
    /// A column, `(N, 1)`.
    Column,
    /// A one-dimensional masked array, `(N,)`, its NaN values masked.
    MaskedRow,
}

/// One direction of a function scale: a function the program gave, or
/// one pltrs makes itself (the spacing of a colorbar's blocks).
pub(crate) trait ScaleFunction: fmt::Debug + Any {
    /// The function's value at `value`.
    fn call(&self, value: f64) -> f64;

    /// Replaces every value of `values` by the function's value there: one
    /// call of a function of the program's taking arrays, handed them laid
    /// out as `layout` says.
    fn call_all(&self, values: &mut [f64], layout: ArrayLayout) {
        let _ = layout;
        for value in values {
            *value = self.call(*value);
        }
    }

    /// Whether the function is code of the program's (see
    /// [`AxisMap::calls_program`]).
    fn calls_program(&self) -> bool {
        true
    }
}

/// The function `x ↦ x`.
#[derive(Debug)]
pub(crate) struct Identity;

pltrs_core::archive_struct!(Identity);

impl ScaleFunction for Identity {
    fn call(&self, value: f64) -> f64 {
        value
    }

    fn calls_program(&self) -> bool {
        false
    }
}

/// A function the program gave.
impl ScaleFunction for ProgramFn<dyn Fn(f64) -> f64> {
    fn call(&self, value: f64) -> f64 {
        (self.get())(value)
    }
}

/// A function the program gave mapping a slice of values in place,
/// handed them laid out as the layout given says.
type ArraysFn = ProgramFn<dyn Fn(&mut [f64], ArrayLayout)>;

/// A function the program gave that maps arrays in place, as
/// matplotlib's `FuncTransform` hands its functions arrays.
impl ScaleFunction for ArraysFn {
    fn call(&self, value: f64) -> f64 {
        let mut one = [value];
        (self.get())(&mut one, ArrayLayout::Row);
        one[0]
    }

    fn call_all(&self, values: &mut [f64], layout: ArrayLayout) {
        (self.get())(values, layout);
    }
}

pltrs_core::archive_kinds!(dyn ScaleFunction {
    "identity" => Identity,
    "program" => ProgramFn<dyn Fn(f64) -> f64>,
    "program-arrays" => ArraysFn,
    "boundaries" => crate::colorbar::BoundarySpacing,
    "norm" => crate::colorbar::NormSpacing,
});

/// A user-supplied forward/inverse pair (matplotlib's `FuncTransform`). A
/// figure with one is archived only if its functions have origins
/// ([`FuncTransform::with_origins`]).
#[derive(Clone)]
pub struct FuncTransform {
    forward: Rc<dyn ScaleFunction>,
    inverse: Rc<dyn ScaleFunction>,
}

pltrs_core::archive_struct!(FuncTransform { forward, inverse });

impl FuncTransform {
    /// `forward` must be monotonic and `inverse` its inverse. Both run
    /// while figures are laid out and drawn: they are `Send`, so they
    /// cannot use a figure's handles.
    pub fn new(
        forward: impl Fn(f64) -> f64 + Send + 'static,
        inverse: impl Fn(f64) -> f64 + Send + 'static,
    ) -> Self {
        Self::from_functions(
            Rc::new(ProgramFn::<dyn Fn(f64) -> f64>::new(Rc::new(forward))),
            Rc::new(ProgramFn::<dyn Fn(f64) -> f64>::new(Rc::new(inverse))),
        )
    }

    /// The pair [`new`](Self::new) makes, archived as `origins`, what the
    /// program made `forward` and `inverse` from (see [`ProgramFn`]).
    pub fn with_origins(
        forward: impl Fn(f64) -> f64 + Send + 'static,
        inverse: impl Fn(f64) -> f64 + Send + 'static,
        origins: (External, External),
    ) -> Self {
        let (forward_origin, inverse_origin) = origins;
        Self::from_functions(
            Rc::new(ProgramFn::<dyn Fn(f64) -> f64>::with_origin(
                Rc::new(forward),
                forward_origin,
            )),
            Rc::new(ProgramFn::<dyn Fn(f64) -> f64>::with_origin(
                Rc::new(inverse),
                inverse_origin,
            )),
        )
    }

    /// A pair of functions mapping arrays in place, each called once for
    /// all the values a path, a set of ticks or a limit needs (matplotlib
    /// hands its `FuncTransform` arrays, laid out as the [`ArrayLayout`]
    /// given says), archived as `origins`, what the program made them
    /// from (see [`ProgramFn`]).
    ///
    /// ```
    /// use pltrs::scale::{ArrayLayout, FuncTransform};
    /// use pltrs::transforms::AxisMap;
    /// use pltrs_core::archive::External;
    /// let double = FuncTransform::on_arrays(
    ///     |values: &mut [f64], _: ArrayLayout| values.iter_mut().for_each(|v| *v *= 2.0),
    ///     |values: &mut [f64], _: ArrayLayout| values.iter_mut().for_each(|v| *v /= 2.0),
    ///     (External::new("forward"), External::new("inverse")),
    /// );
    /// let mut values = [1.0, 2.0, 3.0];
    /// double.forward_all(&mut values);
    /// assert_eq!(values, [2.0, 4.0, 6.0]);
    /// assert_eq!(double.inverse(3.0), 1.5);
    /// ```
    pub fn on_arrays(
        forward: impl Fn(&mut [f64], ArrayLayout) + Send + 'static,
        inverse: impl Fn(&mut [f64], ArrayLayout) + Send + 'static,
        origins: (External, External),
    ) -> Self {
        let (forward_origin, inverse_origin) = origins;
        Self::from_functions(
            Rc::new(ArraysFn::with_origin(Rc::new(forward), forward_origin)),
            Rc::new(ArraysFn::with_origin(Rc::new(inverse), inverse_origin)),
        )
    }

    /// The pair `forward`, `inverse`.
    pub(crate) fn from_functions(
        forward: Rc<dyn ScaleFunction>,
        inverse: Rc<dyn ScaleFunction>,
    ) -> Self {
        Self { forward, inverse }
    }

    /// The identity.
    pub fn identity() -> Self {
        Self::from_functions(Rc::new(Identity), Rc::new(Identity))
    }

    /// The same pair with forward and inverse swapped.
    #[must_use]
    pub fn reversed(&self) -> Self {
        Self {
            forward: Rc::clone(&self.inverse),
            inverse: Rc::clone(&self.forward),
        }
    }
}

impl fmt::Debug for FuncTransform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FuncTransform").finish_non_exhaustive()
    }
}

impl AxisMap for FuncTransform {
    fn forward(&self, value: f64) -> f64 {
        self.forward.call(value)
    }

    fn inverse(&self, value: f64) -> f64 {
        self.inverse.call(value)
    }

    fn forward_all(&self, values: &mut [f64]) {
        self.forward.call_all(values, ArrayLayout::Row);
    }

    fn inverse_all(&self, values: &mut [f64]) {
        self.inverse.call_all(values, ArrayLayout::Row);
    }

    fn forward_column(&self, values: &mut [f64]) {
        self.forward.call_all(values, ArrayLayout::Column);
    }

    fn forward_masked_row(&self, values: &mut [f64]) {
        self.forward.call_all(values, ArrayLayout::MaskedRow);
    }

    fn inverse_column(&self, values: &mut [f64]) {
        self.inverse.call_all(values, ArrayLayout::Column);
    }

    fn calls_program(&self) -> bool {
        self.forward.calls_program() || self.inverse.calls_program()
    }
}

/// A function followed by a logarithm (`FuncTransform + LogTransform`).
#[derive(Clone, Debug)]
pub struct FuncLogTransform {
    func: FuncTransform,
    log: LogTransform,
}

pltrs_core::archive_struct!(FuncLogTransform { func, log });

impl FuncLogTransform {
    /// `log_base(func(x))`.
    pub fn new(func: FuncTransform, log: LogTransform) -> Self {
        Self { func, log }
    }

    /// The log stage.
    pub fn log(&self) -> &LogTransform {
        &self.log
    }
}

impl AxisMap for FuncLogTransform {
    fn forward(&self, value: f64) -> f64 {
        self.log.forward(self.func.forward(value))
    }

    fn forward_masked(&self, value: f64) -> f64 {
        self.log.forward_masked(self.func.forward(value))
    }

    fn inverse(&self, value: f64) -> f64 {
        self.func.inverse(self.log.inverse(value))
    }

    /// The function handed the values as a column, then the logarithm:
    /// matplotlib's composite of the two maps its values through the
    /// function's `transform` (`_a.transform(values)`), which shapes them
    /// so.
    fn forward_all(&self, values: &mut [f64]) {
        self.func.forward_column(values);
        self.log.forward_all(values);
    }

    /// The inverse of the logarithm, then of the function, handed the
    /// values as a column (see [`forward_all`](Self::forward_all)).
    fn inverse_all(&self, values: &mut [f64]) {
        self.log.inverse_all(values);
        self.func.inverse_column(values);
    }

    fn forward_column(&self, values: &mut [f64]) {
        self.func.forward_column(values);
        self.log.forward_all(values);
    }

    fn inverse_column(&self, values: &mut [f64]) {
        self.log.inverse_all(values);
        self.func.inverse_column(values);
    }

    fn calls_program(&self) -> bool {
        self.func.calls_program()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_clip_and_mask() {
        let clip = LogTransform::new(10.0, NonPositive::Clip).unwrap();
        assert_eq!(clip.forward(0.001), -3.0);
        assert_eq!(clip.forward(0.0), -1000.0);
        assert_eq!(clip.forward(-1.0), -1000.0);
        assert!(clip.forward(f64::NAN).is_nan());
        let mask = LogTransform::new(10.0, NonPositive::Mask).unwrap();
        assert_eq!(mask.forward(0.0), f64::NEG_INFINITY);
        assert!(mask.forward(-1.0).is_nan());
        assert!((clip.inverse(2.0) - 100.0).abs() < 1e-12);
        assert!(LogTransform::new(1.0, NonPositive::Clip).is_err());
    }

    #[test]
    fn masked_arrays_lose_the_values_outside_the_domain() {
        let log = LogTransform::new(10.0, NonPositive::Clip).unwrap();
        assert!(log.forward_masked(0.0).is_nan());
        assert!(log.forward_masked(-1.0).is_nan());
        assert_eq!(log.forward_masked(100.0), 2.0);
        let logit = LogitTransform::new(NonPositive::Clip);
        assert!(logit.forward_masked(0.0).is_nan());
        assert!(logit.forward_masked(1.0).is_nan());
        assert_eq!(logit.forward_masked(0.5), 0.0);
        let symlog = SymmetricalLogTransform::new(10.0, 1.0, 1.0).unwrap();
        assert_eq!(symlog.forward_masked(0.0), 0.0, "symlog masks nothing");
    }

    #[test]
    fn symlog_round_trip() {
        let t = SymmetricalLogTransform::new(10.0, 1.0, 1.0).unwrap();
        for v in [-1000.0, -2.0, -0.5, 0.0, 0.5, 2.0, 1000.0] {
            assert!((t.inverse(t.forward(v)) - v).abs() <= 1e-9 * v.abs().max(1.0));
        }
        assert_eq!(t.forward(0.5), 0.5 / 0.9);
        assert!(SymmetricalLogTransform::new(1.0, 1.0, 1.0).is_err());
        assert!(SymmetricalLogTransform::new(10.0, 0.0, 1.0).is_err());
    }

    #[test]
    fn logit_and_asinh() {
        let t = LogitTransform::new(NonPositive::Clip);
        assert_eq!(t.forward(0.5), 0.0);
        assert_eq!(t.forward(1.0), 1000.0);
        assert_eq!(t.inverse(0.0), 0.5);
        let a = AsinhTransform::new(1.0).unwrap();
        assert_eq!(a.forward(0.0), 0.0);
        assert!(AsinhTransform::new(0.0).is_err());
    }

    #[test]
    fn function_maps() {
        let f = FuncTransform::new(|x| 2.0 * x + 1.0, |y| (y - 1.0) / 2.0);
        assert_eq!(f.forward(1.0), 3.0);
        let log = FuncLogTransform::new(f, LogTransform::new(10.0, NonPositive::Clip).unwrap());
        assert_eq!(log.forward(4.5), 1.0);
        assert!((log.inverse(1.0) - 4.5).abs() < 1e-12);
    }
}
