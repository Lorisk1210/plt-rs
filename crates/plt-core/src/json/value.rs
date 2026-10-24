//! The JSON value tree, its accessors and its conversions from Rust values.

/// A parsed JSON value; object members keep their document order.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// A number written with a fraction or exponent (Python's `float`), or
    /// an integer literal too large for [`Json::Integer`].
    Number(f64),
    /// A number written without fraction or exponent (Python's `int`).
    Integer(i64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object with its members in document order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// The member `key` of an object (the first one, if repeated).
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The number, if this is one (integers included).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// The number as an integer, if it is integral and in range.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => self
                .as_f64()
                .filter(|n| n.fract() == 0.0 && n.abs() < 9.0e15)
                .map(|n| n as i64),
        }
    }

    /// Whether the value was written as an integer literal (a Python `int`).
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The members, if this is an object.
    pub fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// Whether this is `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// An array of numbers, if every element is a number.
    pub fn as_f64_vec(&self) -> Option<Vec<f64>> {
        self.as_array()?.iter().map(Self::as_f64).collect()
    }
}

impl From<f64> for Json {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl<T: Into<Self>> From<Vec<T>> for Json {
    fn from(items: Vec<T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Self>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}
