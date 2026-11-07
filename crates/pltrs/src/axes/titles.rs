//! The titles of an axes: left, centre and right (matplotlib's
//! `set_title`), their text, height and gap above the axes.

use std::rc::Rc;

use pltrs_text::{HAlign, VAlign};

use super::{Axes, AxesState, TitlePlacement};
use crate::artist::{ArtistHandle, RawHandle};
use crate::rc::ColorOrAuto;
use crate::rc::keyword_names;
use crate::text::Text;
use crate::transforms::Transform;

/// Where an axes title sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitleLocation {
    /// Flush left.
    Left,
    /// Centred.
    #[default]
    Center,
    /// Flush right.
    Right,
}

pltrs_core::archive_enum!(TitleLocation {
    Left,
    Center,
    Right,
});

impl TitleLocation {
    /// Every location, in the order of the axes' titles.
    const ALL: [Self; 3] = [Self::Left, Self::Center, Self::Right];

    /// matplotlib's name (`left`, `center` or `right`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
        }
    }

    /// The horizontal alignment of a title here (matplotlib aligns titles
    /// by their location name).
    fn halign(self) -> HAlign {
        match self {
            Self::Left => HAlign::Left,
            Self::Center => HAlign::Center,
            Self::Right => HAlign::Right,
        }
    }
}

keyword_names!(TitleLocation, "title location", Self::ALL);

impl Axes {
    /// Sets the title at the `axes.titlelocation` position (centre by
    /// default; matplotlib's `set_title`) and returns it.
    pub fn set_title(&self, text: impl Into<String>) -> Text {
        self.set_title_at(crate::rc::params().axes_titlelocation, text)
    }

    /// The title shown, and where: the one at `axes.titlelocation` if
    /// set, else the first set at the centre, left or right (empty at
    /// `axes.titlelocation` when none is).
    pub fn shown_title(&self) -> (TitleLocation, String) {
        let usual = crate::rc::params().axes_titlelocation;
        [
            usual,
            TitleLocation::Center,
            TitleLocation::Left,
            TitleLocation::Right,
        ]
        .into_iter()
        .map(|location| (location, self.title(location)))
        .find(|(_, title)| !title.is_empty())
        .unwrap_or((usual, String::new()))
    }

    /// Sets the title at `location` and returns it. Like matplotlib's
    /// `set_title`, this resets the gap of all titles to `axes.titlepad`,
    /// the title's height to `axes.titley` (which, when set, also stops
    /// lifting the titles above decorations on top) and its size, weight,
    /// colour and alignment to the `axes.title*` defaults; change them on
    /// the returned title, or with [`Axes::set_title_pad`] and
    /// [`Axes::set_title_y`], afterwards.
    pub fn set_title_at(&self, location: TitleLocation, text: impl Into<String>) -> Text {
        let rc = crate::rc::params();
        self.set_title_pad(rc.axes_titlepad);
        let title = self.title_text(location);
        match rc.axes_titley {
            Some(y) => {
                self.set_title_y(location, y);
            }
            None => {
                title.set_y(1.0);
            }
        }
        title
            .set_text(text)
            .set_fontsize(rc.axes_titlesize)
            .set_weight(rc.axes_titleweight)
            .set_valign(VAlign::Baseline)
            .set_halign(location.halign());
        if let ColorOrAuto::Color(color) = &rc.axes_titlecolor {
            title.set_color(color.clone());
        }
        title
    }

    /// Places the title at `location` at height `y` in axes fractions and
    /// stops lifting this axes' titles above decorations on top
    /// (matplotlib's `set_title(..., y=...)`).
    pub fn set_title_y(&self, location: TitleLocation, y: f64) -> &Self {
        self.title_text(location).set_y(y);
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.title_placement = TitlePlacement::Fixed);
        self
    }

    /// The title at `location` (empty when unset).
    pub fn title(&self, location: TitleLocation) -> String {
        self.title_text(location).text()
    }

    /// The title text at `location`, to style or hide it (matplotlib's
    /// `ax.title`, `ax._left_title` and `ax._right_title`).
    pub fn title_text(&self, location: TitleLocation) -> Text {
        let index = match location {
            TitleLocation::Left => 0,
            TitleLocation::Center => 1,
            TitleLocation::Right => 2,
        };
        let id = self.raw.with::<AxesState, _>(|a| a.titles[index]);
        Text::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), id))
    }

    /// Sets the gap between the axes top and its titles in points
    /// (matplotlib's `set_title(..., pad=...)`).
    pub fn set_title_pad(&self, pad: f64) -> &Self {
        for location in TitleLocation::ALL {
            self.title_text(location)
                .set_transform(Transform::Axes(self.id()).offset_points(0.0, pad));
        }
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.title_pad = pad);
        self
    }

    /// The gap between the axes top and its titles in points, as
    /// [`Axes::set_title_pad`] or `axes.titlepad` set it (the offset of
    /// matplotlib's `titleOffsetTrans`).
    pub fn title_pad(&self) -> f64 {
        self.raw.with::<AxesState, _>(|axes| axes.title_pad)
    }

    /// Lifts the titles above what rises over the top of the axes
    /// (matplotlib's `_update_title_position`): tick labels or an axis
    /// label on top, a y offset text in the way. Titles placed at a fixed
    /// height stay put. This lays the figure out as drawing does, which
    /// places the titles of every axes once their ticks are known.
    pub fn update_title_position(&self) -> &Self {
        // A hook stopping the layout leaves the titles where they are.
        let _ = self.figure().prepare_layout();
        self
    }

    /// How the height of the titles is chosen: lifted above the
    /// decorations on top as the figure is laid out (the default), at a
    /// fixed height (`axes.titley`, [`Axes::set_title_y`]), or where the
    /// program puts them.
    pub fn title_placement(&self) -> TitlePlacement {
        self.raw.with::<AxesState, _>(|axes| axes.title_placement)
    }

    /// Chooses how the height of the titles is chosen (see
    /// [`Axes::title_placement`]). With [`TitlePlacement::Program`] the
    /// titles stay where the program puts them, which lifts them with
    /// [`Axes::lift_titles`] when it wants them clear of the decorations,
    /// as an `Axes` subclass of matplotlib's placing its titles its own
    /// way calls `super()._update_title_position(renderer)` first.
    ///
    /// ```
    /// use pltrs::axes::TitlePlacement;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.xaxis().tick_to(SpineSide::Top)?;
    /// let title = ax.set_title("above the tick labels");
    /// ax.set_title_placement(TitlePlacement::Program);
    /// fig.layout();
    /// assert_eq!(title.position().y, 1.0, "where the program left it");
    /// ax.lift_titles();
    /// assert!(title.position().y > 1.0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_title_placement(&self, placement: TitlePlacement) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.title_placement = placement);
        self
    }

    /// Lifts the titles above what rises over the top of the axes now,
    /// the axes laid out first, however their height is chosen
    /// ([`Axes::title_placement`]): titles at a fixed height stay put.
    /// Unlike [`Axes::update_title_position`], this lays out no other
    /// axes and runs no layout engine.
    pub fn lift_titles(&self) -> &Self {
        self.raw.figure_state().work_on(|state| {
            state.prepare();
            state.lift_titles(self.id());
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_round_trip_their_names() {
        for location in TitleLocation::ALL {
            assert_eq!(location.to_string().parse(), Ok(location));
        }
        assert!("top".parse::<TitleLocation>().is_err());
    }

    #[test]
    fn the_title_pad_follows_the_rc_and_set_title_pad() {
        let ax = crate::figure::Figure::new().subplot();
        assert_eq!(ax.title_pad(), crate::rc::params().axes_titlepad);
        ax.set_title_pad(30.0);
        assert_eq!(ax.title_pad(), 30.0);
        assert_eq!(
            ax.title_text(TitleLocation::Left).transform(),
            Some(Transform::Axes(ax.id()).offset_points(0.0, 30.0))
        );
        ax.set_title("again");
        assert_eq!(ax.title_pad(), crate::rc::params().axes_titlepad);
    }
}
