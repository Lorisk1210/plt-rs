//! Panicking accessors for the fixture files, which `plt_core::json`
//! reads (Python's `json` writes them, `NaN` and `Infinity` included): a
//! fixture that does not have the expected shape fails the test.

pub(crate) use plt_core::json::Json;

/// Reads `text`, failing the test on a malformed fixture.
pub(crate) fn parse(text: &str) -> Json {
    Json::parse(text).unwrap_or_else(|err| panic!("malformed fixture: {err}"))
}

/// The accessors the fixture comparisons read values with.
pub(crate) trait Fixture {
    /// Object member `key`.
    fn field(&self, key: &str) -> &Json;
    /// Whether this is an object with member `key`.
    fn has(&self, key: &str) -> bool;
    /// The number.
    fn f64(&self) -> f64;
    /// The number, truncated to an index.
    fn usize(&self) -> usize;
    /// The boolean.
    fn bool(&self) -> bool;
    /// The string.
    fn str(&self) -> &str;
    /// The array's elements.
    fn array(&self) -> &[Json];
    /// The array's elements as numbers.
    fn f64_vec(&self) -> Vec<f64>;
}

impl Fixture for Json {
    fn field(&self, key: &str) -> &Json {
        self.get(key)
            .unwrap_or_else(|| panic!("missing key {key:?}"))
    }

    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn f64(&self) -> f64 {
        self.as_f64()
            .unwrap_or_else(|| panic!("expected a number, got {self:?}"))
    }

    fn usize(&self) -> usize {
        self.f64() as usize
    }

    fn bool(&self) -> bool {
        self.as_bool()
            .unwrap_or_else(|| panic!("expected a bool, got {self:?}"))
    }

    fn str(&self) -> &str {
        self.as_str()
            .unwrap_or_else(|| panic!("expected a string, got {self:?}"))
    }

    fn array(&self) -> &[Json] {
        self.as_array()
            .unwrap_or_else(|| panic!("expected an array, got {self:?}"))
    }

    fn f64_vec(&self) -> Vec<f64> {
        self.array().iter().map(Json::f64).collect()
    }
}
