//! Figure-wide texts, legends and layout of the current figure
//! (matplotlib's `suptitle`, `figtext`, `figlegend`, `subplots_adjust`
//! and `tight_layout`).

use super::registry::gcf;
use crate::gridspec::{GridSpecError, SubplotOverrides};
use crate::layout::{TightLayout, TightLayoutError};
use crate::legend::{Legend, LegendError, LegendOptions};
use crate::text::Text;

/// Sets the title of the current figure (matplotlib's `suptitle`).
pub fn suptitle(text: impl Into<String>) -> Text {
    gcf().suptitle(text)
}

/// Adds a text at `(x, y)` in fractions of the current figure
/// (matplotlib's `figtext`, an alias of
/// [`Figure::text`](crate::figure::Figure::text)).
pub fn figtext(x: f64, y: f64, text: impl Into<String>) -> Text {
    gcf().text(x, y, text)
}

/// Places a legend on the current figure, listing the labelled artists of
/// all its axes unless given handles (matplotlib's `figlegend`, which is
/// `gcf().legend`).
///
/// # Errors
///
/// As [`Figure::legend`](crate::figure::Figure::legend).
pub fn figlegend(options: LegendOptions) -> Result<Legend, LegendError> {
    gcf().legend(options)
}

/// Changes the subplot margins and spacing of the current figure
/// (matplotlib's `subplots_adjust`).
///
/// # Errors
///
/// When the margins leave no room or the spacing is negative.
pub fn subplots_adjust(overrides: &SubplotOverrides) -> Result<(), GridSpecError> {
    gcf().subplots_adjust(overrides).map(|_| ())
}

/// Fits the subplots of the current figure and their decorations into it
/// (matplotlib's `tight_layout`).
///
/// # Errors
///
/// As [`Figure::tight_layout`](crate::figure::Figure::tight_layout).
pub fn tight_layout(params: &TightLayout) -> Result<(), TightLayoutError> {
    gcf().tight_layout(params).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::pyplot::{close_all, figure, plot, subplot};

    #[test]
    fn figure_texts_and_legends() {
        close_all();
        let fig = figure();
        let caption = figtext(0.5, 0.02, "caption");
        assert!(fig.contains_artist(&caption));
        assert_eq!(caption.text(), "caption");
        suptitle("overall");
        assert_eq!(fig.suptitle_text(), "overall");
        plot(vec![0.0, 1.0], vec![0.0, 1.0])
            .unwrap()
            .set_label("data");
        let legend = figlegend(LegendOptions::default()).unwrap();
        assert_eq!(fig.legends(), [legend]);
        close_all();
    }

    #[test]
    fn layouts_move_the_subplots() {
        close_all();
        let ax = subplot(2, 1, 1).unwrap();
        subplot(2, 1, 2).unwrap();
        let before = ax.position();
        subplots_adjust(&SubplotOverrides {
            top: Some(0.8),
            ..SubplotOverrides::default()
        })
        .unwrap();
        assert!(ax.position().y1() < before.y1());
        assert!(
            subplots_adjust(&SubplotOverrides {
                left: Some(0.9),
                right: Some(0.1),
                ..SubplotOverrides::default()
            })
            .is_err()
        );
        tight_layout(&TightLayout::default()).unwrap();
        close_all();
    }
}
