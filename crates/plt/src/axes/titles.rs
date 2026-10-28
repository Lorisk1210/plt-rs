//! The titles of an axes: left, centre and right (matplotlib's
//! `set_title`), their text, height and gap above the axes.

use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

use plt_text::{HAlign, VAlign};

use super::{Axes, AxesState, state};
use crate::artist::{ArtistHandle, RawHandle};
use crate::image::UnknownName;
use crate::rc::ColorOrAuto;
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

impl fmt::Display for TitleLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for TitleLocation {
    type Err = UnknownName;

    /// Parses matplotlib's name ([`TitleLocation::name`]).
    fn from_str(name: &str) -> Result<Self, UnknownName> {
        Self::ALL
            .into_iter()
            .find(|location| location.name() == name)
            .ok_or_else(|| UnknownName {
                kind: "title location",
                name: name.to_owned(),
            })
    }
}

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
            .with_mut::<AxesState, _>(|axes| axes.title_placement = state::TitlePlacement::Fixed);
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
}
