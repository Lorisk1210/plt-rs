//! numpy's arithmetic on its float scalars (`np.float64`), with the
//! runtime warnings it issues where the IEEE results are invalid, divide
//! by zero or overflow: matplotlib's code computing with them warns so,
//! which pltrs reproduces where it computes the same values (the matrices
//! of the box transforms; see
//! [`warn_runtime`](crate::warnings::warn_runtime)); and numpy's product
//! of the 3×3 matrices matplotlib composes affine transforms with, with
//! the warnings of the floating-point exceptions its BLAS raises.

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
    let mut raised = Exceptions::default();
    for (result, a, b) in elements {
        raised.note(result, a, b);
    }
    raised.warn(name);
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

/// Issues the warnings of numpy's `np.dot(b, a)` of two 3×3 matrices
/// (`invalid value encountered in dot` for an infinity multiplied by a
/// zero or two opposite infinities added, `overflow encountered in dot`),
/// where numpy's products warn at all
/// ([`PRODUCTS_WARN`](crate::warnings::PRODUCTS_WARN)): what matplotlib's
/// composite transforms issue each time they multiply their matrices.
///
/// `OpenBLAS` multiplies two matrices by adding the products of each
/// entry in order, each fused with the sum so far, which starts at zero:
/// a sum that is NaN already hides what the next product raises (the
/// processor's fused multiply-add hands the NaN on and raises nothing).
///
/// ```
/// use pltrs::numeric::numpy_arith::warn_dot3;
/// let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
/// // Finite matrices raise nothing.
/// warn_dot3(&identity, &identity);
/// ```
pub fn warn_dot3(b: &[[f64; 3]; 3], a: &[[f64; 3]; 3]) {
    if !crate::warnings::PRODUCTS_WARN {
        return;
    }
    let mut raised = Exceptions::default();
    for b_row in b {
        for j in 0..3 {
            let mut sum = 0.0;
            for (&factor, a_row) in b_row.iter().zip(a) {
                sum = raised.accumulate(factor, a_row[j], sum);
            }
        }
    }
    raised.warn("dot");
}

/// The floating-point exceptions one numpy call raised, which it reports
/// as warnings when it returns, each kind once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Exceptions {
    /// A finite number divided by zero.
    pub(crate) divide_by_zero: bool,
    /// A finite result too large to represent.
    pub(crate) overflow: bool,
    /// No number made of numbers (`0 × inf`, `inf - inf`, `0 / 0`,
    /// `inf / inf`).
    pub(crate) invalid: bool,
}

impl Exceptions {
    /// Raises the exceptions `result` of an operation on `a` and `b`
    /// raised: NaN from operands that are not NaN is invalid, an infinity
    /// from finite operands divides by zero (for a division by zero) or
    /// overflows.
    pub(crate) fn note(&mut self, result: f64, a: f64, b: f64) {
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

    /// `x × y + sum` as one fused operation, noting the exceptions it
    /// raises, as `OpenBLAS` accumulates the entries of a matrix product:
    /// zero times an infinity is invalid unless `sum` is NaN already (the
    /// processor's fused multiply-add then hands the NaN on and raises
    /// nothing), as are a product and a sum of opposite infinities; only
    /// the result overflows.
    pub(crate) fn accumulate(&mut self, x: f64, y: f64, sum: f64) -> f64 {
        let result = x.mul_add(y, sum);
        let product = x * y;
        if product.is_nan() && !x.is_nan() && !y.is_nan() {
            self.invalid |= !sum.is_nan();
        } else if product.is_infinite() && sum.is_infinite() && product.signum() != sum.signum() {
            self.invalid = true;
        }
        self.overflow |= result.is_infinite() && x.is_finite() && y.is_finite() && sum.is_finite();
        result
    }

    /// `x × y + sum` (`x × y` without `sum`), noting the exceptions
    /// computing it raises where the product and the sum are computed one
    /// after the other (`OpenBLAS`'s product of a matrix and a vector).
    pub(crate) fn multiply_add(&mut self, x: f64, y: f64, sum: Option<f64>) -> f64 {
        let product = x * y;
        if (x == 0.0 && y.is_infinite()) || (x.is_infinite() && y == 0.0) {
            self.invalid = true;
        }
        let Some(sum) = sum else {
            self.overflow |= product.is_infinite() && x.is_finite() && y.is_finite();
            return product;
        };
        let result = x.mul_add(y, sum);
        if product.is_infinite() && sum.is_infinite() && product.signum() != sum.signum() {
            self.invalid = true;
        }
        self.overflow |= result.is_infinite() && x.is_finite() && y.is_finite() && sum.is_finite();
        result
    }

    /// `a / b`, noting the exceptions dividing raises.
    pub(crate) fn divide(&mut self, a: f64, b: f64) -> f64 {
        let quotient = a / b;
        self.note(quotient, a, b);
        quotient
    }

    /// Whether any exception is raised.
    fn raised(self) -> bool {
        self.divide_by_zero || self.overflow || self.invalid
    }

    /// Issues numpy's warnings for the exceptions `operation` (`"scalar
    /// multiply"`, `"dot"`) raised, in the order numpy checks them.
    pub(crate) fn warn(self, operation: &str) {
        for (raised, problem) in [
            (self.divide_by_zero, "divide by zero"),
            (self.overflow, "overflow"),
            (self.invalid, "invalid value"),
        ] {
            if raised {
                crate::warnings::warn_runtime(&format!("{problem} encountered in {operation}"));
            }
        }
    }
}

/// `result` of numpy's scalar operation `name` on `a` and `b`, with its
/// warning.
fn scalar(name: &str, result: f64, a: f64, b: f64) -> f64 {
    let mut raised = Exceptions::default();
    raised.note(result, a, b);
    if raised.raised() {
        raised.warn(&format!("scalar {name}"));
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

    /// `warnings`, without those of products where numpy's products warn
    /// of nothing.
    fn as_numpy_warns(warnings: &[&str]) -> Vec<String> {
        warnings
            .iter()
            .filter(|_| crate::warnings::PRODUCTS_WARN)
            .map(|&warning| warning.to_owned())
            .collect()
    }

    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    #[test]
    fn products_of_finite_matrices_and_of_nan_are_quiet() {
        let scaled = [[2.0, 0.0, 5.0], [0.0, 3.0, 7.0], [0.0, 0.0, 1.0]];
        let nan = [[f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let quiet = warnings_of(|| {
            warn_dot3(&scaled, &IDENTITY);
            warn_dot3(&scaled, &nan);
            warn_dot3(&nan, &scaled);
        });
        assert!(quiet.is_empty(), "{quiet:?}");
    }

    #[test]
    fn an_infinity_times_a_zero_is_invalid_in_dot_once() {
        let infinite = [[f64::INFINITY, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let scaled = [[2.0, 0.0, 5.0], [0.0, 3.0, 7.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            warnings_of(|| warn_dot3(&scaled, &infinite)),
            as_numpy_warns(&["invalid value encountered in dot"])
        );
        assert_eq!(
            warnings_of(|| warn_dot3(&infinite, &scaled)),
            as_numpy_warns(&["invalid value encountered in dot"])
        );
    }

    #[test]
    fn opposite_infinities_added_are_invalid_and_large_products_overflow() {
        let right = [[1.0, 0.0, f64::INFINITY], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let left = [
            [1.0, 0.0, f64::NEG_INFINITY],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        assert_eq!(
            warnings_of(|| warn_dot3(&right, &left)),
            as_numpy_warns(&["invalid value encountered in dot"])
        );
        let large = [[1e308, 0.0, 1e308], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let tenfold = [[10.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            warnings_of(|| warn_dot3(&tenfold, &large)),
            as_numpy_warns(&["overflow encountered in dot"])
        );
    }

    #[test]
    fn a_nan_sum_hides_the_products_after_it() {
        // Row `[2, 0, 3]` times the column `[NaN, inf, NaN]`: NaN first.
        let row = [[2.0, 0.0, 3.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let column = |c: [f64; 3]| [[1.0, c[0], 0.0], [0.0, c[1], 0.0], [0.0, c[2], 1.0]];
        let nan_first = column([f64::NAN, f64::INFINITY, f64::NAN]);
        assert_eq!(
            warnings_of(|| warn_dot3(&row, &nan_first)),
            Vec::<String>::new()
        );
        let nan_last = column([1.0, f64::INFINITY, f64::NAN]);
        assert_eq!(
            warnings_of(|| warn_dot3(&row, &nan_last)),
            as_numpy_warns(&["invalid value encountered in dot"])
        );
        let opposite = column([f64::INFINITY, f64::NAN, f64::NEG_INFINITY]);
        let full = [[2.0, 2.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            warnings_of(|| warn_dot3(&full, &opposite)),
            as_numpy_warns(&["invalid value encountered in dot"])
        );
    }

    #[test]
    fn opposite_infinities_in_a_sum_are_invalid() {
        let mut exceptions = Exceptions::default();
        let sum = exceptions.multiply_add(1.0, f64::INFINITY, None);
        exceptions.multiply_add(-1.0, f64::INFINITY, Some(sum));
        assert!(exceptions.invalid && !exceptions.overflow);
    }

    #[test]
    fn finite_products_too_large_overflow() {
        let mut exceptions = Exceptions::default();
        exceptions.multiply_add(1e200, 1e200, None);
        assert!(exceptions.overflow && !exceptions.invalid);
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
