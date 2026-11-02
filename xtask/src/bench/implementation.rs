//! The three implementations the benchmark compares.

/// matplotlib, or one of the two faces of pltrs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Implementation {
    /// matplotlib in the reference venv.
    Matplotlib,
    /// pltrs (Python): the same scripts with their imports changed.
    Python,
    /// pltrs (Rust): the same figures written against the `pltrs` crate.
    Rust,
}

impl Implementation {
    /// All three, matplotlib first.
    pub(super) const ALL: [Self; 3] = [Self::Matplotlib, Self::Python, Self::Rust];

    /// The two measured against matplotlib.
    pub(super) const CONTENDERS: [Self; 2] = [Self::Python, Self::Rust];

    /// Its key in the raw data, in labels and in artifact directories.
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Matplotlib => "matplotlib",
            Self::Python => "pltrs-python",
            Self::Rust => "pltrs-rust",
        }
    }

    /// Its name in the report.
    pub(super) const fn title(self) -> &'static str {
        match self {
            Self::Matplotlib => "matplotlib",
            Self::Python => "pltrs (Python)",
            Self::Rust => "pltrs (Rust)",
        }
    }
}

/// One value per implementation.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct PerImplementation<T> {
    /// matplotlib's.
    pub(super) matplotlib: T,
    /// pltrs (Python)'s.
    pub(super) python: T,
    /// pltrs (Rust)'s.
    pub(super) rust: T,
}

impl<T> PerImplementation<T> {
    /// The value of `implementation`.
    pub(super) const fn get(&self, implementation: Implementation) -> &T {
        match implementation {
            Implementation::Matplotlib => &self.matplotlib,
            Implementation::Python => &self.python,
            Implementation::Rust => &self.rust,
        }
    }

    /// The value of `implementation`, to change.
    pub(super) const fn get_mut(&mut self, implementation: Implementation) -> &mut T {
        match implementation {
            Implementation::Matplotlib => &mut self.matplotlib,
            Implementation::Python => &mut self.python,
            Implementation::Rust => &mut self.rust,
        }
    }

    /// The values `f` makes of each implementation.
    pub(super) fn from_fn(mut f: impl FnMut(Implementation) -> T) -> Self {
        Self {
            matplotlib: f(Implementation::Matplotlib),
            python: f(Implementation::Python),
            rust: f(Implementation::Rust),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_name_the_implementations() {
        let values = PerImplementation::from_fn(Implementation::key);
        assert_eq!(*values.get(Implementation::Python), "pltrs-python");
    }
}
