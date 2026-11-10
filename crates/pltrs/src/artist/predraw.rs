//! The calls of the program's code that shape artists (its box styles)
//! in a draw that renders nothing: matplotlib draws a figure once before
//! it prints it cut to its tight box or laid out by an engine (with
//! drawing disabled), which pltrs does not paint, and every artist drawn
//! there calls the styles it is drawn with (see
//! [`Artist::program_shaping`](super::Artist::program_shaping)).

use super::{ArtistId, ArtistLookup, LayoutContext, RawHandle};
use crate::axes::AxesState;
use crate::figure::FigureState;

/// A call of the program's code shaping an artist, to make with the
/// figure free (see [`Artist::program_shaping`](super::Artist::program_shaping)).
pub type ProgramShaping = Box<dyn FnOnce()>;

impl RawHandle {
    /// Makes the calls of the program's code the artists under this one
    /// (and it) make to shape themselves in a draw that renders nothing,
    /// in drawing order, with the figure lent meanwhile.
    pub(crate) fn predraw_program_shapes(&self) {
        let figure = self.figure_state();
        figure.work_on(|state| {
            let calls = shapings_under(state, self.id());
            if !calls.is_empty() {
                state.lend(|_| calls.into_iter().for_each(|call| call()));
            }
        });
    }
}

/// The calls shaping the artists a draw of `root` reaches (visible ones,
/// children by z-order, animated ones left out as the figure and its axes
/// leave them out), in drawing order.
fn shapings_under(state: &FigureState, root: ArtistId) -> Vec<ProgramShaping> {
    let layout = LayoutContext::new(state);
    let mut calls = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        let Some(artist) = state.artist(id) else {
            continue;
        };
        if !artist.base().visible {
            continue;
        }
        calls.extend(artist.program_shaping(&layout));
        let in_axes = state.store.get_as::<AxesState>(id).is_some();
        let mut children: Vec<(f64, ArtistId)> = artist
            .children()
            .into_iter()
            .filter(|&child| !layout.skips_animated(child, in_axes))
            .filter_map(|child| Some((state.artist(child)?.base().zorder, child)))
            .collect();
        children.sort_by(|a, b| a.0.total_cmp(&b.0));
        pending.extend(children.into_iter().rev().map(|(_, child)| child));
    }
    calls
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::figure::{BboxInches, Figure, SaveOptions};
    use crate::patches::{BoxShape, BoxStyle, ProgramBox};
    use crate::path::Path;
    use crate::text::TextBoxProps;
    use crate::transforms::Point;

    #[test]
    fn prints_drawn_before_call_the_program_s_box_styles_again() {
        // matplotlib 3.11: a text box of a style of the program's is shaped
        // once per draw, twice in a print cut to its tight box.
        let calls = Rc::new(Cell::new(0));
        let counted = Rc::clone(&calls);
        let style = ProgramBox::new(Rc::new(move |x0, y0, width, height, _| {
            counted.set(counted.get() + 1);
            Path::polygon([(x0, y0), (x0 + width, y0), (x0 + width, y0 + height)].map(Point::from))
        }));
        let fig = Figure::new();
        let text = fig.text(0.5, 0.5, "boxed");
        let props = TextBoxProps {
            boxstyle: Some(BoxStyle::new(BoxShape::Program(style))),
            ..TextBoxProps::default()
        };
        text.set_bbox(Some(props));
        fig.to_png().unwrap();
        assert_eq!(calls.replace(0), 1);
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Tight),
            ..SaveOptions::default()
        };
        fig.render_raster_with(tight).unwrap();
        assert_eq!(calls.get(), 2);
    }
}
