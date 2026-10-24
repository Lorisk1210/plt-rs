//! Labels added after [`ContourSet::clabel`]: snapped to the line nearest
//! to a point, put exactly where they are told, and removed again
//! (matplotlib's `add_label_near`, `add_label` and `pop_label`).

use std::rc::Rc;

use plt_core::Point;

use super::placer::{LabelSetup, Placer};
use crate::artist::{ArtistHandle, RawHandle};
use crate::contour::{ContourError, ContourSet};
use crate::text::Text;
use crate::transforms::Transform;

impl ContourSet {
    /// Labels the labelled line nearest to `point`, given in the
    /// coordinates of `transform` (data: the axes' `trans_data`; display
    /// pixels: [`Transform::Display`]), the way the last
    /// [`ContourSet::clabel`] call labelled the set, and returns the new
    /// text (matplotlib's `add_label_near`). With `inline` the line is cut
    /// under the label, leaving `inline_spacing` pixels on each side.
    ///
    /// # Errors
    ///
    /// When the set has not been labelled yet, `transform` cannot be
    /// resolved, or there is no labelled line to snap to.
    pub fn add_label_near(
        &self,
        point: Point,
        transform: &Transform,
        inline: bool,
        inline_spacing: f64,
    ) -> Result<Text, ContourError> {
        let setup = self.label_setup()?;
        let target = self.to_display(point, transform)?;
        Placer::new(self, &setup)?.label_near(target, inline, inline_spacing)
    }

    /// Adds a label of the level at index `level`, centred at `point` in
    /// the coordinates of `transform` and turned by `rotation` degrees
    /// counter-clockwise in the display, the way the last
    /// [`ContourSet::clabel`] call labelled the set, and returns its text
    /// (matplotlib's `add_label`). Unlike [`ContourSet::add_label_near`]
    /// it neither snaps the label to a line nor cuts one.
    ///
    /// # Errors
    ///
    /// When the set has not been labelled yet, `level` is out of range or
    /// was not labelled by that call, or `transform` cannot be resolved.
    pub fn add_label(
        &self,
        point: Point,
        transform: &Transform,
        rotation: f64,
        level: usize,
    ) -> Result<Text, ContourError> {
        let setup = self.label_setup()?;
        let levels = self.levels();
        let value = *levels.get(level).ok_or(ContourError::LevelIndex {
            index: level,
            levels: levels.len(),
        })?;
        let label = setup
            .labels
            .iter()
            .find(|label| label.index == level)
            .ok_or_else(|| ContourError::UnknownLabelLevels(vec![value]))?;
        let at = self.to_display(point, transform)?;
        Ok(Placer::new(self, &setup)?.add(label, at, rotation))
    }

    /// Removes the label at `index` in [`ContourSet::labels`], or the last
    /// one when `None`, with its text (matplotlib's `pop_label`). Lines cut
    /// under an inline label stay cut, as in matplotlib.
    ///
    /// # Errors
    ///
    /// When there is no label at `index`, or no label at all.
    pub fn pop_label(&self, index: Option<usize>) -> Result<(), ContourError> {
        let record = self.update(|state| {
            let at = match index {
                Some(i) if i < state.labels.len() => i,
                None if !state.labels.is_empty() => state.labels.len() - 1,
                _ => return Err(ContourError::NoLabel(index)),
            };
            Ok(state.labels.remove(at))
        })?;
        let figure = Rc::clone(self.axes().raw().figure_state());
        Text::from_raw(RawHandle::new(figure, record.artist)).remove();
        Ok(())
    }

    /// How the last [`ContourSet::clabel`] call labelled the set.
    pub(super) fn label_setup(&self) -> Result<LabelSetup, ContourError> {
        self.read(|state| state.label_setup.clone())
            .ok_or(ContourError::Unlabelled)
    }

    /// The display position of `point`, given in the coordinates of
    /// `transform`.
    pub(super) fn to_display(
        &self,
        point: Point,
        transform: &Transform,
    ) -> Result<Point, ContourError> {
        let figure = self.axes().raw().figure_state().borrow();
        Ok(transform.resolve(&*figure)?.transform_point(point))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::bowl;
    use super::super::{ClabelFormat, ClabelOptions};
    use super::*;

    #[test]
    fn labels_added_near_a_point_follow_the_last_clabel() {
        let cs = bowl(&[0.5, 1.0]);
        let data = cs.axes().trans_data();
        assert_eq!(
            cs.add_label_near(Point::new(0.7, 0.0), &data, true, 5.0)
                .unwrap_err(),
            ContourError::Unlabelled
        );
        let options = ClabelOptions {
            fmt: ClabelFormat::Map(vec![(0.5, "half".to_owned()), (1.0, "one".to_owned())]),
            ..ClabelOptions::default()
        };
        let automatic = cs.clabel(options).unwrap().len();
        let near = cs
            .add_label_near(Point::new(0.0, 0.98), &data, true, 5.0)
            .unwrap();
        let labels = cs.labels();
        assert_eq!(labels.len(), automatic + 1);
        let last = &labels[labels.len() - 1];
        assert_eq!((last.level, last.text.as_str()), (1.0, "one"));
        assert_eq!(last.artist, near);
        // A display point labels the line nearest to it on screen.
        let display = data
            .resolve(&*cs.axes().raw().figure_state().borrow())
            .unwrap()
            .transform_point(Point::new(0.7, 0.0));
        cs.add_label_near(display, &Transform::Display, false, 5.0)
            .unwrap();
        assert_eq!(cs.labels().last().unwrap().level, 0.5);
    }

    #[test]
    fn labels_go_exactly_where_they_are_told() {
        let cs = bowl(&[0.5, 1.0]);
        let data = cs.axes().trans_data();
        let at = Point::new(0.2, -0.3);
        assert_eq!(
            cs.add_label(at, &data, 30.0, 1).unwrap_err(),
            ContourError::Unlabelled
        );
        let options = ClabelOptions {
            levels: Some(vec![1.0]),
            manual: Some(Vec::new()),
            ..ClabelOptions::default()
        };
        cs.clabel(options).unwrap();
        let paths = cs.paths();
        let text = cs.add_label(at, &data, 30.0, 1).unwrap();
        let position = text.position();
        assert!((position.x - at.x).abs() < 1e-12 && (position.y - at.y).abs() < 1e-12);
        assert_eq!(text.rotation(), 30.0);
        let labels = cs.labels();
        assert_eq!(
            (labels.len(), labels[0].level, labels[0].rotation),
            (1, 1.0, 30.0)
        );
        assert_eq!(cs.paths(), paths, "no line is cut");
        assert_eq!(
            cs.add_label(at, &data, 0.0, 0).unwrap_err(),
            ContourError::UnknownLabelLevels(vec![0.5]),
            "level 0.5 was not labelled"
        );
        assert_eq!(
            cs.add_label(at, &data, 0.0, 2).unwrap_err(),
            ContourError::LevelIndex {
                index: 2,
                levels: 2
            }
        );
    }

    #[test]
    fn popped_labels_lose_their_text() {
        let cs = bowl(&[0.5, 1.0]);
        assert_eq!(cs.pop_label(None).unwrap_err(), ContourError::NoLabel(None));
        let options = ClabelOptions {
            manual: Some(vec![(0.7, 0.0), (0.0, 0.98)]),
            ..ClabelOptions::default()
        };
        let texts = cs.clabel(options).unwrap();
        assert_eq!(
            cs.pop_label(Some(2)).unwrap_err(),
            ContourError::NoLabel(Some(2))
        );
        cs.pop_label(None).unwrap();
        let labels = cs.labels();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].artist, texts[0]);
        assert!(texts[0].raw().is_attached() && !texts[1].raw().is_attached());
        cs.pop_label(Some(0)).unwrap();
        assert!(cs.labels().is_empty() && !texts[0].raw().is_attached());
    }
}
