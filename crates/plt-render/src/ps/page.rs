//! The page a PostScript document is printed on and how the figure sits
//! on it (matplotlib's `papertype` and `orientation`).

use std::fmt;
use std::str::FromStr;

/// matplotlib's paper sizes in inches (`backend_ps.papersize`).
const PAPER_SIZES: [(&str, f64, f64); 25] = [
    ("letter", 8.5, 11.0),
    ("legal", 8.5, 14.0),
    ("ledger", 11.0, 17.0),
    ("a0", 33.11, 46.81),
    ("a1", 23.39, 33.11),
    ("a2", 16.54, 23.39),
    ("a3", 11.69, 16.54),
    ("a4", 8.27, 11.69),
    ("a5", 5.83, 8.27),
    ("a6", 4.13, 5.83),
    ("a7", 2.91, 4.13),
    ("a8", 2.05, 2.91),
    ("a9", 1.46, 2.05),
    ("a10", 1.02, 1.46),
    ("b0", 40.55, 57.32),
    ("b1", 28.66, 40.55),
    ("b2", 20.27, 28.66),
    ("b3", 14.33, 20.27),
    ("b4", 10.11, 14.33),
    ("b5", 7.16, 10.11),
    ("b6", 5.04, 7.16),
    ("b7", 3.58, 5.04),
    ("b8", 2.51, 3.58),
    ("b9", 1.76, 2.51),
    ("b10", 1.26, 1.76),
];

/// The page a PostScript document is printed on (`ps.papersize`, or
/// `savefig`'s `papertype`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paper {
    /// A page the size of the figure.
    Figure,
    /// A named paper size, in inches (portrait).
    Named {
        /// matplotlib's name, e.g. `"letter"` or `"a4"`.
        name: &'static str,
        /// Width in inches.
        width: f64,
        /// Height in inches.
        height: f64,
    },
}

impl Paper {
    /// The paper called `name` (case-insensitive), or `"figure"`.
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        if name == "figure" {
            return Some(Self::Figure);
        }
        PAPER_SIZES
            .iter()
            .find(|(n, _, _)| *n == name)
            .map(|&(name, width, height)| Self::Named {
                name,
                width,
                height,
            })
    }
}

impl FromStr for Paper {
    type Err = PageSetupError;

    /// Parses a paper name as [`Paper::from_name`] does; matplotlib's
    /// former `"auto"` is refused like any unknown name.
    ///
    /// ```
    /// use plt_render::Paper;
    /// assert!(matches!("A4".parse(), Ok(Paper::Named { name: "a4", .. })));
    /// assert_eq!("figure".parse(), Ok(Paper::Figure));
    /// assert!("auto".parse::<Paper>().is_err());
    /// ```
    fn from_str(name: &str) -> Result<Self, PageSetupError> {
        Self::from_name(name).ok_or_else(|| PageSetupError::Papertype(name.to_owned()))
    }
}

/// How the figure sits on the page (`savefig.orientation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageOrientation {
    /// Upright.
    #[default]
    Portrait,
    /// Rotated by 90°.
    Landscape,
}

impl PageOrientation {
    /// matplotlib's name of the orientation.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
        }
    }
}

impl FromStr for PageOrientation {
    type Err = PageSetupError;

    /// Parses `"portrait"` or `"landscape"`.
    ///
    /// ```
    /// use plt_render::PageOrientation;
    /// assert_eq!("landscape".parse(), Ok(PageOrientation::Landscape));
    /// assert!("sideways".parse::<PageOrientation>().is_err());
    /// ```
    fn from_str(name: &str) -> Result<Self, PageSetupError> {
        match name {
            "portrait" => Ok(Self::Portrait),
            "landscape" => Ok(Self::Landscape),
            _ => Err(PageSetupError::Orientation(name.to_owned())),
        }
    }
}

/// A paper or orientation name that does not exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageSetupError {
    /// An unknown paper name.
    Papertype(String),
    /// An unknown orientation.
    Orientation(String),
}

impl fmt::Display for PageSetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Papertype(name) => {
                write!(
                    f,
                    "{name:?} is not a valid value for papertype; supported values are \"figure\""
                )?;
                PAPER_SIZES
                    .iter()
                    .try_for_each(|(paper, _, _)| write!(f, ", {paper:?}"))
            }
            Self::Orientation(name) => write!(
                f,
                "{name:?} is not a valid value for orientation; supported values are \"portrait\", \"landscape\""
            ),
        }
    }
}

impl std::error::Error for PageSetupError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_names_are_refused() {
        let auto = "auto".parse::<Paper>().unwrap_err();
        assert!(auto.to_string().starts_with(
            "\"auto\" is not a valid value for papertype; supported values are \"figure\", \"letter\""
        ));
        assert_eq!(
            "Landscape".parse::<PageOrientation>(),
            Err(PageSetupError::Orientation("Landscape".to_owned()))
        );
        assert_eq!(PageOrientation::Landscape.name(), "landscape");
    }
}
