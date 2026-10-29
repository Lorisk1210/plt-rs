//! Panicking accessors for the fixture files, which `pltrs_core::json`
//! reads: a fixture that does not have the expected shape fails the test.

pub(crate) use pltrs_core::json::Json;

/// Reads `text`, failing the test on malformed JSON.
pub(crate) fn parse(text: &str) -> Json {
    Json::parse(text).unwrap_or_else(|err| panic!("malformed fixture: {err}"))
}

/// The accessors the fixture comparisons read values with.
pub(crate) trait Fixture {
    /// Object member `key`.
    fn field(&self, key: &str) -> &Json;
    /// Object member `key`, unless it is absent or `null`.
    fn opt(&self, key: &str) -> Option<&Json>;
    /// The array's elements.
    fn array(&self) -> &[Json];
    /// The object's members, in document order.
    fn object(&self) -> &[(String, Json)];
    /// The number.
    fn f64(&self) -> f64;
    /// The number, truncated to an integer.
    fn i64(&self) -> i64;
    /// The string.
    fn str(&self) -> &str;
    /// The boolean.
    fn bool(&self) -> bool;
    /// The array's elements as truncated integers.
    fn i64s(&self) -> Vec<i64>;
}

impl Fixture for Json {
    fn field(&self, key: &str) -> &Json {
        self.get(key).unwrap_or_else(|| panic!("missing key {key}"))
    }

    fn opt(&self, key: &str) -> Option<&Json> {
        self.get(key).filter(|value| !value.is_null())
    }

    fn array(&self) -> &[Json] {
        self.as_array()
            .unwrap_or_else(|| panic!("not an array: {self:?}"))
    }

    fn object(&self) -> &[(String, Json)] {
        self.as_object()
            .unwrap_or_else(|| panic!("not an object: {self:?}"))
    }

    fn f64(&self) -> f64 {
        self.as_f64()
            .unwrap_or_else(|| panic!("not a number: {self:?}"))
    }

    fn i64(&self) -> i64 {
        self.f64() as i64
    }

    fn str(&self) -> &str {
        self.as_str()
            .unwrap_or_else(|| panic!("not a string: {self:?}"))
    }

    fn bool(&self) -> bool {
        self.as_bool()
            .unwrap_or_else(|| panic!("not a bool: {self:?}"))
    }

    fn i64s(&self) -> Vec<i64> {
        self.array().iter().map(Json::i64).collect()
    }
}
