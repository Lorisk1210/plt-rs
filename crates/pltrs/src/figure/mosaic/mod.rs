//! Named subplot layouts (matplotlib's `subplot_mosaic`): a [`Mosaic`] of
//! labelled cells laid out as one axes per label.

mod cells;
mod error;

pub use cells::{Mosaic, MosaicCell};
pub use error::MosaicError;

use super::{Figure, MosaicAxes};
use crate::axes::{Axes, AxisName};
use crate::gridspec::{GridSpec, SubplotOverrides};
use crate::projections::{ProjectionError, ProjectionSpec};
use cells::Region;

/// Options of [`Figure::subplot_mosaic`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MosaicOptions {
    /// Relative column widths of the outer grid.
    pub width_ratios: Option<Vec<f64>>,
    /// Relative row heights of the outer grid.
    pub height_ratios: Option<Vec<f64>>,
    /// Share the x axis of every axes with the first one.
    pub sharex: bool,
    /// Share the y axis of every axes with the first one.
    pub sharey: bool,
    /// The projection of every axes, with its options (matplotlib's
    /// `subplot_kw={"projection": ..., **options}`); rectilinear by
    /// default.
    pub projection: ProjectionSpec,
    /// The projections of the axes with these labels, in place of
    /// [`projection`](Self::projection) (matplotlib's
    /// `per_subplot_kw={label: {"projection": ..., **options}}`); each
    /// label may appear once.
    pub per_subplot: Vec<(String, ProjectionSpec)>,
    /// The margins and spacing of the outer grid (matplotlib's
    /// `gridspec_kw` `left`, `right`, `bottom`, `top`, `wspace` and
    /// `hspace`).
    pub gridspec: SubplotOverrides,
}

impl MosaicOptions {
    /// The projection of the axes labelled `label`.
    fn projection_of(&self, label: &str) -> ProjectionSpec {
        self.per_subplot
            .iter()
            .find(|(name, _)| name == label)
            .map_or(&self.projection, |(_, projection)| projection)
            .clone()
    }
}

impl Figure {
    /// Adds one axes per label of `mosaic` (matplotlib's
    /// `subplot_mosaic`), created level by level in order of their
    /// top-left cells. The mosaic is a [`Mosaic`], a reference to one, or
    /// text it parses.
    ///
    /// ```
    /// use pltrs::figure::{Figure, MosaicOptions};
    /// let fig = Figure::new();
    /// let axes = fig.subplot_mosaic("AAB;C.B", MosaicOptions::default())?;
    /// let labels: Vec<&str> = axes.iter().map(|(label, _)| label).collect();
    /// assert_eq!(labels, ["A", "B", "C"]);
    /// # Ok::<(), pltrs::figure::MosaicError>(())
    /// ```
    ///
    /// Axes of registered projections, with options, go per label:
    ///
    /// ```
    /// use pltrs::axes::ProjectionKind;
    /// use pltrs::figure::{Figure, MosaicOptions};
    /// let options = MosaicOptions {
    ///     projection: ProjectionKind::Polar.into(),
    ///     per_subplot: vec![("C".into(), ProjectionKind::Mollweide.into())],
    ///     ..MosaicOptions::default()
    /// };
    /// let axes = Figure::new().subplot_mosaic("AB;CC", options)?;
    /// let kinds: Vec<&str> = axes.iter().map(|(_, ax)| ax.projection().name()).collect();
    /// assert_eq!(kinds, ["polar", "polar", "mollweide"]);
    /// # Ok::<(), pltrs::figure::MosaicError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the text of the mosaic does not parse, a label covers a
    /// non-rectangular area or appears twice, the ratios do not match the
    /// grid, a label of
    /// [`per_subplot`](MosaicOptions::per_subplot) is not in the mosaic or
    /// is listed twice, or
    /// axes cannot be made with a projection the options ask for
    /// (as [`Figure::subplots_on_with_projection`]: unknown projections and
    /// options add no axes).
    pub fn subplot_mosaic(
        &self,
        mosaic: impl TryInto<Mosaic, Error: Into<MosaicError>>,
        options: MosaicOptions,
    ) -> Result<MosaicAxes, MosaicError> {
        let mosaic = &mosaic.try_into().map_err(Into::into)?;
        if let Some((label, _)) = options
            .per_subplot
            .iter()
            .find(|(label, _)| !mosaic.contains(label))
        {
            return Err(MosaicError::UnknownLabel(label.clone()));
        }
        for (index, (label, _)) in options.per_subplot.iter().enumerate() {
            if options.per_subplot[..index]
                .iter()
                .any(|(earlier, _)| earlier == label)
            {
                return Err(MosaicError::RepeatedLabel(label.clone()));
            }
        }
        options.projection.check()?;
        for (_, projection) in &options.per_subplot {
            projection.check()?;
        }
        let (rows, cols) = mosaic.shape();
        let grid = GridSpec::with_ratios(
            rows,
            cols,
            options.width_ratios.clone(),
            options.height_ratios.clone(),
            options.gridspec,
        )?;
        let mut entries = Vec::new();
        self.lay_out_mosaic(&grid, mosaic, &options, &mut entries)?;
        if let Some((_, first)) = entries.first().cloned() {
            for (share, name) in [(options.sharex, AxisName::X), (options.sharey, AxisName::Y)] {
                if !share {
                    continue;
                }
                for (_, ax) in &entries {
                    ax.share(name, &first).map_err(ProjectionError::Share)?;
                    ax.label_outer_shared(name);
                }
            }
        }
        Ok(MosaicAxes::new(entries))
    }

    fn lay_out_mosaic(
        &self,
        grid: &GridSpec,
        mosaic: &Mosaic,
        options: &MosaicOptions,
        entries: &mut Vec<(String, Axes)>,
    ) -> Result<(), MosaicError> {
        for region in mosaic.regions()? {
            match region {
                Region::Axes { label, rows, cols } => {
                    if entries.iter().any(|(name, _)| name == label) {
                        return Err(MosaicError::Duplicate(label.to_owned()));
                    }
                    let spec = grid.span(rows, cols)?;
                    let ax =
                        self.add_subplot_spec_with_projection(&spec, options.projection_of(label))?;
                    entries.push((label.to_owned(), ax));
                }
                Region::Nested { row, col, inner } => {
                    let (rows, cols) = inner.shape();
                    let sub = GridSpec::nested(
                        &grid.cell(row, col)?,
                        rows,
                        cols,
                        None,
                        None,
                        (None, None),
                    )?;
                    self.lay_out_mosaic(&sub, inner, options, entries)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ProjectionKind;
    use crate::gridspec::GridSpecError;

    #[test]
    fn labels_must_be_rectangles() {
        let fig = Figure::new();
        let options = MosaicOptions::default();
        let bent = "AA;A.".parse::<Mosaic>().unwrap();
        assert_eq!(
            fig.subplot_mosaic(&bent, options).err(),
            Some(MosaicError::NonRectangular("A".into()))
        );
    }

    #[test]
    fn nested_mosaics_and_sharing() {
        let fig = Figure::new();
        let inner = "ab;cd".parse::<Mosaic>().unwrap();
        let mosaic = Mosaic::new(vec![
            vec![MosaicCell::Label("main".into()), MosaicCell::Nested(inner)],
            vec![MosaicCell::Label("bottom".into()); 2],
        ])
        .unwrap();
        let options = MosaicOptions {
            sharex: true,
            ..MosaicOptions::default()
        };
        let axes = fig.subplot_mosaic(&mosaic, options).unwrap();
        let labels: Vec<&str> = axes.iter().map(|(label, _)| label).collect();
        assert_eq!(labels, ["main", "a", "b", "c", "d", "bottom"]);
        let main = axes.get("main").unwrap();
        main.set_xlim(0.0, 5.0).expect("finite limits");
        assert_eq!(axes.get("d").unwrap().xlim(), (0.0, 5.0));
    }

    #[test]
    fn labelled_axes_turn_polar_and_the_grid_takes_its_margins() {
        let fig = Figure::new();
        let options = MosaicOptions {
            per_subplot: vec![("B".into(), ProjectionKind::Polar.into())],
            gridspec: SubplotOverrides {
                left: Some(0.6),
                right: Some(0.95),
                ..SubplotOverrides::default()
            },
            ..MosaicOptions::default()
        };
        let axes = fig
            .subplot_mosaic("AB".parse::<Mosaic>().unwrap(), options)
            .unwrap();
        let kind = |label: &str| axes.get(label).unwrap().projection();
        assert_eq!(
            (kind("A"), kind("B")),
            (ProjectionKind::Rectilinear, ProjectionKind::Polar)
        );
        let (left, _, _, _) = axes.get("A").unwrap().position().bounds();
        assert!((left - 0.6).abs() < 1e-12, "{left}");
    }

    #[test]
    fn per_subplot_labels_must_be_in_the_mosaic() {
        // matplotlib's test_extra_per_subplot_kw.
        let fig = Figure::new();
        let options = MosaicOptions {
            per_subplot: vec![("B".into(), ProjectionKind::Polar.into())],
            ..MosaicOptions::default()
        };
        assert_eq!(
            fig.subplot_mosaic("A".parse::<Mosaic>().unwrap(), options),
            Err(MosaicError::UnknownLabel("B".into()))
        );
        assert!(fig.axes().is_empty(), "no axes is made");
        let nested = "AB".parse::<Mosaic>().unwrap();
        assert!(nested.contains("B") && !nested.contains("C"));
    }

    #[test]
    fn per_subplot_labels_may_appear_once() {
        // matplotlib's test_per_subplot_kw_expander.
        let fig = Figure::new();
        let options = MosaicOptions {
            per_subplot: vec![
                ("A".into(), ProjectionKind::Polar.into()),
                ("B".into(), ProjectionKind::Polar.into()),
                ("B".into(), ProjectionKind::Rectilinear.into()),
            ],
            ..MosaicOptions::default()
        };
        assert_eq!(
            fig.subplot_mosaic("AB".parse::<Mosaic>().unwrap(), options),
            Err(MosaicError::RepeatedLabel("B".into()))
        );
        assert!(fig.axes().is_empty(), "no axes is made");
    }

    #[test]
    fn projections_the_mosaic_cannot_make_add_no_axes() {
        let fig = Figure::new();
        let mosaic = "AB".parse::<Mosaic>().unwrap();
        let unknown = MosaicOptions {
            per_subplot: vec![("B".into(), ProjectionKind::Registered("nowhere").into())],
            ..MosaicOptions::default()
        };
        assert_eq!(
            fig.subplot_mosaic(&mosaic, unknown),
            Err(MosaicError::Projection(ProjectionError::Unknown(
                "nowhere".into()
            )))
        );
        let with_options = MosaicOptions {
            projection: ProjectionKind::Hammer.with_options(1.0),
            ..MosaicOptions::default()
        };
        assert!(matches!(
            fig.subplot_mosaic(&mosaic, with_options),
            Err(MosaicError::Projection(ProjectionError::Options { .. }))
        ));
        assert_eq!(fig.axes(), []);
        assert_eq!(
            MosaicError::from(ProjectionError::Grid(GridSpecError::EmptyGrid)),
            MosaicError::Grid(GridSpecError::EmptyGrid)
        );
    }
}
