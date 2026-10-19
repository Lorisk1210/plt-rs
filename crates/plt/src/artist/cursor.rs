//! What a status bar shows of the data under the pointer (matplotlib's
//! `Artist.format_cursor_data`; colour-mapped values are formatted by
//! their mappable's `format_cursor_data`).

use crate::ticker::pyfmt;

/// The significant digits of plain values (matplotlib's `0.3g`).
const DEFAULT_DIGITS: usize = 3;

/// Plain values as `[v1, v2]`, three significant digits each (matplotlib's
/// `Artist.format_cursor_data`).
pub(crate) fn format_values(values: impl IntoIterator<Item = f64>) -> String {
    let items: Vec<String> = values
        .into_iter()
        .map(|value| pyfmt::general(value, DEFAULT_DIGITS))
        .collect();
    format!("[{}]", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_get_three_digits() {
        assert_eq!(
            format_values([0.123_456, 255.0, 1234.0]),
            "[0.123, 255, 1.23e+03]"
        );
        assert_eq!(format_values([]), "[]");
    }
}
