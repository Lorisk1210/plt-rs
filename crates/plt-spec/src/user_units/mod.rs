//! Unit classes written against plt's public unit API
//! ([`plt::units::ConversionInterface`] and [`plt::units::Convertible`])
//! as matplotlib's tests and gallery write them in Python: the units
//! tests' `Quantity`, the parts of `matplotlib.testing.jpl_units` they use,
//! the gallery's `basic_units`, and the spec's own `Temperature`.

pub(crate) mod basic_units;
pub(crate) mod jpl;
pub(crate) mod quantity;
pub(crate) mod temperature;
