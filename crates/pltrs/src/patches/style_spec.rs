//! Parsing of style descriptions such as `"round,pad=0.2"` (matplotlib's
//! `_Style.__new__`), shared by box, arrow and connection styles.

use std::fmt;

use super::StyleError;

/// A style name with its numeric `key=value` attributes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StyleSpec {
    /// The style name, lower-cased.
    pub(crate) name: String,
    attributes: Vec<(String, f64)>,
}

impl StyleSpec {
    /// Splits `text` at commas after removing spaces: the first item is the
    /// name, the others `key=value` pairs with numeric values.
    pub(crate) fn parse(text: &str) -> Result<Self, StyleError> {
        let compact: String = text.chars().filter(|&c| c != ' ').collect();
        let mut items = compact.split(',');
        let name = items.next().unwrap_or_default().to_lowercase();
        let attributes = items
            .map(|item| {
                let (key, value) = item
                    .split_once('=')
                    .filter(|(_, value)| !value.contains('='))
                    .ok_or_else(|| StyleError::BadAttribute(text.to_owned()))?;
                let value = value
                    .parse::<f64>()
                    .map_err(|_| StyleError::BadAttribute(text.to_owned()))?;
                Ok((key.to_owned(), value))
            })
            .collect::<Result<_, StyleError>>()?;
        Ok(Self { name, attributes })
    }

    /// Removes and returns the attribute `key` (the last one if repeated).
    pub(crate) fn take(&mut self, key: &str) -> Option<f64> {
        let mut value = None;
        self.attributes.retain(|(k, v)| {
            let matches = k == key;
            if matches {
                value = Some(*v);
            }
            !matches
        });
        value
    }

    /// Fails if an attribute was not taken by the style.
    pub(crate) fn finish(self) -> Result<(), StyleError> {
        match self.attributes.into_iter().next() {
            Some((parameter, _)) => Err(StyleError::UnknownParameter {
                style: self.name,
                parameter,
            }),
            None => Ok(()),
        }
    }

    /// The error for a name no style answers to.
    pub(crate) fn unknown(&self) -> StyleError {
        StyleError::UnknownStyle(self.name.clone())
    }
}

/// Writes a style's description, which [`StyleSpec::parse`] reads back:
/// `name`, then `,key=value` for each attribute that has a value (the
/// style strings of matplotlib's `_Style`, `"round,pad=0.2"`).
pub(crate) fn write_description(
    f: &mut fmt::Formatter<'_>,
    name: &str,
    attributes: &[(&str, Option<f64>)],
) -> fmt::Result {
    f.write_str(name)?;
    for (key, value) in attributes {
        if let Some(value) = value {
            write!(f, ",{key}={value}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_attributes() {
        let mut spec = StyleSpec::parse("Round , pad = 0.2,rounding_size=1e-1").unwrap();
        assert_eq!(spec.name, "round");
        assert_eq!(spec.take("pad"), Some(0.2));
        assert_eq!(spec.take("pad"), None);
        assert_eq!(spec.take("rounding_size"), Some(0.1));
        assert!(spec.finish().is_ok());
    }

    #[test]
    fn malformed_attributes() {
        assert!(StyleSpec::parse("round,pad").is_err());
        assert!(StyleSpec::parse("round,pad=x").is_err());
        assert!(StyleSpec::parse("round,pad=1=2").is_err());
        let spec = StyleSpec::parse("round,size=2").unwrap();
        assert!(matches!(
            spec.finish(),
            Err(StyleError::UnknownParameter { .. })
        ));
        assert_eq!(
            StyleSpec::parse("").unwrap().unknown(),
            StyleError::UnknownStyle(String::new())
        );
    }
}
