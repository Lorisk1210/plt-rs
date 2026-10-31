//! The conventions of a locale for writing numbers, as Python's
//! `locale.localeconv()` gives them: a dict of `decimal_point`,
//! `thousands_sep` and `grouping`, whose sizes are counts of digits, `0`
//! repeating the previous one and `locale.CHAR_MAX` ending grouping.

use pltrs::ticker::{Grouping, NumericLocale};

use crate::binding::prelude::*;
use crate::binding::{dict_value, new_dict};

/// Python's `locale.CHAR_MAX`, which ends a grouping.
const CHAR_MAX: usize = 127;

/// A [`NumericLocale`] as a `localeconv()` dict.
pub(super) struct PyLocale(pub(super) NumericLocale);

impl FromPy<'_> for PyLocale {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let grouping = dict_value::<Vec<usize>>(object, "grouping")?
            .into_iter()
            .map(|size| match size {
                0 => Grouping::RepeatLast,
                CHAR_MAX => Grouping::Stop,
                size => Grouping::Size(size),
            })
            .collect();
        Ok(Self(NumericLocale {
            decimal_point: dict_value(object, "decimal_point")?,
            thousands_sep: dict_value(object, "thousands_sep")?,
            grouping,
        }))
    }
}

impl IntoPy for PyLocale {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let NumericLocale {
            decimal_point,
            thousands_sep,
            grouping,
        } = self.0;
        let grouping: Vec<usize> = grouping
            .into_iter()
            .map(|size| match size {
                Grouping::RepeatLast => 0,
                Grouping::Stop => CHAR_MAX,
                Grouping::Size(size) => size,
            })
            .collect();
        let conv = new_dict(py)?;
        conv.set_item("decimal_point", decimal_point)?;
        conv.set_item("thousands_sep", thousands_sep)?;
        conv.set_item("grouping", grouping)?;
        Ok(conv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, with_python};

    #[test]
    fn localeconv_dicts_round_trip() {
        with_python(|py| {
            let conv = eval(
                py,
                "",
                "{'decimal_point': ',', 'thousands_sep': '.', 'grouping': [3, 0]}",
            );
            let PyLocale(locale) = conv.extract().unwrap();
            assert_eq!(locale.grouping, [Grouping::Size(3), Grouping::RepeatLast]);
            assert_eq!(locale.localize("1234.5"), "1.234,5");
            let back = PyLocale(locale.clone()).into_py(py).unwrap();
            let PyLocale(again) = back.extract().unwrap();
            assert_eq!(again, locale);
        });
    }
}
