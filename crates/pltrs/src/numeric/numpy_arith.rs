//! numpy's arithmetic on its float scalars (`np.float64`), with the
//! runtime warnings it issues where the IEEE results are invalid, divide
//! by zero or overflow: matplotlib's code computing with them warns so,
//! which pltrs reproduces where it computes the same values (the matrices
//! of the box transforms; see
//! [`warn_runtime`](crate::warnings::warn_runtime)); and numpy's product
//! of the 3×3 matrices matplotlib composes affine transforms with.

/// `a - b` of numpy scalars, warning as numpy does.
pub fn subtract(a: f64, b: f64) -> f64 {
    scalar("subtract", a - b, a, b)
}

/// `a + b` of numpy scalars, warning as numpy does.
pub fn add(a: f64, b: f64) -> f64 {
    scalar("add", a + b, a, b)
}

/// `a * b` of numpy scalars, warning as numpy does.
pub fn multiply(a: f64, b: f64) -> f64 {
    scalar("multiply", a * b, a, b)
}

/// `a / b` of numpy scalars, warning as numpy does: dividing a number
/// other than zero by zero divides by zero, zero by zero is invalid.
pub fn divide(a: f64, b: f64) -> f64 {
    scalar("divide", a / b, a, b)
}

/// Issues numpy's warnings for one call of the elementwise operation
/// `name` (`"power"`, `"exp"`), each element given as its result and its
/// two operands (a unary operation's operand twice): numpy reports each
/// kind of problem once per call, whichever elements raised it.
///
/// ```
/// use pltrs::numeric::numpy_arith::warn_elementwise;
/// // 10**400 overflows, 10**2 does not: one warning for the call.
/// warn_elementwise("power", [(f64::INFINITY, 10.0, 400.0), (100.0, 10.0, 2.0)]);
/// ```
pub fn warn_elementwise(name: &str, elements: impl IntoIterator<Item = (f64, f64, f64)>) {
    let mut flags = Flags::default();
    for (result, a, b) in elements {
        flags.note(result, a, b);
    }
    flags.warn(name);
}

/// numpy's `np.dot(b, a)` of two 3×3 matrices (rows of columns), each
/// entry the sum of its three products in order, so that a NaN or an
/// infinity reaches every entry it is multiplied into, the bottom row
/// too (matplotlib composes its affine transforms so).
///
/// ```
/// use pltrs::numeric::numpy_arith::dot3;
/// let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
/// let scaled = [[f64::NAN, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
/// let product = dot3(&identity, &scaled);
/// assert!(product[2][0].is_nan(), "0 * NaN reaches the bottom row");
/// assert_eq!(product[2][2], 1.0);
/// ```
pub fn dot3(b: &[[f64; 3]; 3], a: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut product = [[0.0; 3]; 3];
    for (row, b_row) in product.iter_mut().zip(b) {
        for (j, entry) in row.iter_mut().enumerate() {
            *entry = b_row[0] * a[0][j] + b_row[1] * a[1][j] + b_row[2] * a[2][j];
        }
    }
    product
}

/// The floating-point problems of one operation, as numpy's flags hold
/// them.
#[derive(Clone, Copy, Default)]
struct Flags {
    divide_by_zero: bool,
    overflow: bool,
    invalid: bool,
}

impl Flags {
    /// Raises the flags `result` of an operation on `a` and `b` sets: NaN
    /// from operands that are not NaN is invalid, an infinity from finite
    /// operands divides by zero (for a division by zero) or overflows.
    fn note(&mut self, result: f64, a: f64, b: f64) {
        if result.is_nan() && !a.is_nan() && !b.is_nan() {
            self.invalid = true;
        } else if result.is_infinite() && a.is_finite() && b.is_finite() {
            if b == 0.0 {
                self.divide_by_zero = true;
            } else {
                self.overflow = true;
            }
        }
    }

    /// Whether any flag is raised.
    fn raised(self) -> bool {
        self.divide_by_zero || self.overflow || self.invalid
    }

    /// Issues the warnings of the raised flags for the operation `name`
    /// (`"scalar multiply"`), in the order numpy checks them.
    fn warn(self, name: &str) {
        for (raised, problem) in [
            (self.divide_by_zero, "divide by zero"),
            (self.overflow, "overflow"),
            (self.invalid, "invalid value"),
        ] {
            if raised {
                crate::warnings::warn_runtime(&format!("{problem} encountered in {name}"));
            }
        }
    }
}

/// `result` of numpy's scalar operation `name` on `a` and `b`, with its
/// warning.
fn scalar(name: &str, result: f64, a: f64, b: f64) -> f64 {
    let mut flags = Flags::default();
    flags.note(result, a, b);
    if flags.raised() {
        flags.warn(&format!("scalar {name}"));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The runtime warnings `work` issues.
    fn warnings_of(work: impl FnOnce()) -> Vec<String> {
        crate::warnings::capturing_runtime(work).1
    }

    #[test]
    fn invalid_results_warn() {
        assert_eq!(
            warnings_of(|| assert!(subtract(f64::INFINITY, f64::INFINITY).is_nan())),
            ["invalid value encountered in scalar subtract"]
        );
        assert_eq!(
            warnings_of(|| assert!(multiply(f64::INFINITY, 0.0).is_nan())),
            ["invalid value encountered in scalar multiply"]
        );
        assert_eq!(
            warnings_of(|| assert!(divide(0.0, 0.0).is_nan())),
            ["invalid value encountered in scalar divide"]
        );
    }

    #[test]
    fn infinities_from_finite_numbers_warn() {
        assert_eq!(
            warnings_of(|| assert_eq!(divide(1.0, 0.0), f64::INFINITY)),
            ["divide by zero encountered in scalar divide"]
        );
        assert_eq!(
            warnings_of(|| assert_eq!(multiply(1e308, 10.0), f64::INFINITY)),
            ["overflow encountered in scalar multiply"]
        );
    }

    #[test]
    fn elementwise_calls_warn_once_per_kind() {
        assert_eq!(
            warnings_of(|| warn_elementwise(
                "power",
                [
                    (f64::INFINITY, 10.0, 400.0),
                    (f64::INFINITY, 10.0, 500.0),
                    (f64::INFINITY, 10.0, f64::INFINITY),
                ]
            )),
            ["overflow encountered in power"]
        );
        assert_eq!(
            warnings_of(|| warn_elementwise("log", [(f64::NEG_INFINITY, 0.0, 0.0)])),
            ["divide by zero encountered in log"]
        );
        assert_eq!(
            warnings_of(|| warn_elementwise("exp", [(1.0, 0.0, 0.0)])),
            Vec::<String>::new()
        );
    }

    #[test]
    fn nan_and_infinite_operands_are_quiet() {
        let quiet = warnings_of(|| {
            assert!(subtract(f64::NAN, 1.0).is_nan());
            assert_eq!(add(f64::INFINITY, 1.0), f64::INFINITY);
            assert_eq!(divide(f64::INFINITY, 0.0), f64::INFINITY);
            assert_eq!(subtract(3.0, 1.0), 2.0);
        });
        assert!(quiet.is_empty(), "{quiet:?}");
    }
}
