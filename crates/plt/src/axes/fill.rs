//! Filled polygons (matplotlib's `fill`).

use plt_core::Point;

use super::children::DataUpdate;
use super::{Axes, AxesState, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape, Polygon};
use crate::rc::CycleValue;
use crate::units::AxisValues;

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
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let triangle = ax.fill([0.0, 1.0, 0.5], [0.0, 0.0, 1.0], PatchProps::default())?;
    /// assert_eq!(triangle.shape().vertices().len(), 4);
    /// # Ok::<(), plt::axes::PlotError>(())
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
        let (x, y) = (self.xaxis().convert_units(x), self.yaxis().convert_units(y));
        let (x, y) = (x.as_slice(), y.as_slice());
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let vertices = x.iter().zip(y).map(|(&x, &y)| Point::new(x, y));
        Ok(self.add_polygon(Polygon::new(vertices, true), props))
    }

    /// Adds `polygon` styled by the property cycle and `props`, extending
    /// the data limits and autoscaling (matplotlib's `_make_polygon`
    /// followed by `add_patch` in `fill`).
    pub(crate) fn add_polygon(&self, polygon: Polygon, props: PatchProps) -> Patch<Polygon> {
        let mut patch = PatchArtist::new(polygon, crate::rc::params());
        for (key, value) in self.polygon_cycle_entry(&props) {
            apply_cycle(&mut patch, &key, value);
        }
        props.apply(&mut patch);
        patch
            .base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        let update = DataUpdate::data(patch.data_points());
        Patch::from_raw(self.adopt(patch, &update))
    }

    /// The next patch property-cycle entry without the properties `props`
    /// sets, or nothing when `props` sets every cycled property
    /// (matplotlib's `_getdefaults` with `fill`'s ignored keys).
    fn polygon_cycle_entry(&self, props: &PatchProps) -> Vec<(String, CycleValue)> {
        self.raw().with_mut::<AxesState, _>(|axes| {
            let ignored = |key: &str| MARKER_KEYS.contains(&key) || props.is_set(key);
            let entry: Vec<(String, CycleValue)> = axes
                .prop_cycle
                .entry(axes.patch_cycle)
                .filter(|(key, _)| !ignored(key))
                .map(|(key, value)| (key.to_owned(), value.clone()))
                .collect();
            if entry.is_empty() {
                return entry;
            }
            axes.patch_cycle += 1;
            entry
        })
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
