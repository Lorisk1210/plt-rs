//! Filled polygons (matplotlib's `fill`).

use pltrs_core::Point;

use super::children::DataUpdate;
use super::{Axes, PlotError, PropCycle};
use crate::artist::Artist;
use crate::colors::Color;
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape, Polygon};
use crate::rc::CycleValue;
use crate::units::{AxisValues, UnitHandle};

/// Line properties a polygon ignores when it takes a property-cycle entry.
const MARKER_KEYS: [&str; 5] = [
    "marker",
    "markersize",
    "markeredgecolor",
    "markerfacecolor",
    "markeredgewidth",
];

impl Axes {
    /// A filled, closed polygon through `(x, y)`, numbers, dates or
    /// categories (matplotlib's `fill`). Its face colour comes from the
    /// property cycle unless `props.color` is set.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let triangle = ax.fill([0.0, 1.0, 0.5], [0.0, 0.0, 1.0], PatchProps::default())?;
    /// assert_eq!(triangle.shape().vertices().len(), 4);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn fill(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        props: PatchProps,
    ) -> Result<Patch<Polygon>, PlotError> {
        self.fill_with_format_color(x, y, None, props)
    }

    /// [`Axes::fill`] with the colour of a format string, if any
    /// (matplotlib's `fill(x, y, "r")`): it fills the polygon in place of
    /// the cycle's colour, so the property cycle does not advance unless
    /// it cycles other properties `props` does not set.
    ///
    /// ```
    /// use pltrs::colors::Rgba;
    /// use pltrs::prelude::*;
    /// let (x, y) = ([0.0, 1.0, 0.5], [0.0, 0.0, 1.0]);
    /// let ax = Figure::new().subplot();
    /// let red = ax.fill_with_format_color(x, y, Some("r".into()), PatchProps::default())?;
    /// let next = ax.fill(x, y, PatchProps::default())?;
    /// let first = Figure::new().subplot().fill(x, y, PatchProps::default())?;
    /// assert_eq!(red.colors()?.face, Rgba::opaque(1.0, 0.0, 0.0));
    /// assert_eq!(next.colors()?.face, first.colors()?.face, "the cycle did not advance");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn fill_with_format_color(
        &self,
        x: impl Into<AxisValues>,
        y: impl Into<AxisValues>,
        format_color: Option<Color>,
        props: PatchProps,
    ) -> Result<Patch<Polygon>, PlotError> {
        let (x, y) = (
            self.xaxis().convert_plot_units(x.into()),
            self.yaxis().convert_plot_units(y.into()),
        );
        let (x, y) = (x.as_slice(), y.as_slice());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let vertices = x.iter().zip(y).map(|(&x, &y)| Point::new(x, y));
        Ok(self.add_formatted_polygon(Polygon::new(vertices, true), format_color, props))
    }

    /// Adds `polygon` styled by the property cycle and `props`, extending
    /// the data limits and autoscaling (matplotlib's `_make_polygon`
    /// followed by `add_patch` in `fill`).
    pub(crate) fn add_polygon(&self, polygon: Polygon, props: PatchProps) -> Patch<Polygon> {
        self.add_formatted_polygon(polygon, None, props)
    }

    /// [`Axes::add_polygon`] filled with the colour of a format string, if
    /// any, which the cycle's colour does not replace.
    fn add_formatted_polygon(
        &self,
        polygon: Polygon,
        format_color: Option<Color>,
        props: PatchProps,
    ) -> Patch<Polygon> {
        let mut patch = PatchArtist::new(polygon, crate::rc::params());
        let color_given = format_color.is_some();
        if color_given {
            patch.style.facecolor = format_color;
        }
        for (key, value) in self.polygon_cycle_entry(&props, color_given) {
            apply_cycle(&mut patch, &key, value);
        }
        props.apply(&mut patch);
        patch
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        let update = DataUpdate::data(patch.data_points());
        Patch::from_raw(self.adopt_in_autoscale(patch, &update))
    }

    /// The next patch property-cycle entry without the properties `props`
    /// sets (and the colour, when `color_given`), or nothing when they
    /// cover every cycled property (matplotlib's `_getdefaults` with
    /// `fill`'s ignored keys).
    fn polygon_cycle_entry(
        &self,
        props: &PatchProps,
        color_given: bool,
    ) -> Vec<(String, CycleValue)> {
        let ignored = |key: &str| {
            MARKER_KEYS.contains(&key) || props.is_set(key) || (color_given && key == "color")
        };
        self.with_cycle(PropCycle::Patches, |cycle| cycle.take_entry(ignored))
    }
}

/// Applies one property-cycle entry to a new polygon; the cycled colour
/// becomes the face colour.
fn apply_cycle<S: PatchShape>(patch: &mut PatchArtist<S>, key: &str, value: CycleValue) {
    let style = &mut patch.style;
    match (key, value) {
        ("color" | "facecolor", CycleValue::Color(color)) => style.facecolor = Some(color),
        ("edgecolor", CycleValue::Color(color)) => style.edgecolor = Some(color),
        ("linestyle", CycleValue::LineStyle(linestyle)) => style.linestyle = linestyle,
        ("linewidth", CycleValue::Number(width)) => style.linewidth = Some(width),
        ("hatch", CycleValue::Text(hatch)) => style.hatch = Some(hatch),
        ("alpha", CycleValue::Number(alpha)) => patch.base_mut().alpha = Some(alpha),
        _ => {}
    }
}
