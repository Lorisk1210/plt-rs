//! matplotlib's toolbar images (`mpl-data/images`), bundled once from
//! `web/assets/images`: the web backend serves them at matplotlib's URLs,
//! and the toolbar of the matplotlib chrome shows them, as matplotlib's
//! Tk toolbar does.

use pltrs_raster::Pixmap;
use pltrs_ui::Picture;

/// The side of the toolbar images in logical pixels: matplotlib's Tk
/// toolbar sizes them at 18 points, 24 pixels at 96 dpi.
const SIDE: f64 = 24.0;

/// One of matplotlib's toolbar images: the PNG for 1× screens, 24 pixels
/// wide, and the `_large` one for 2× screens, 48 pixels wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ToolbarImage {
    /// The 24-pixel PNG.
    pub(crate) normal: &'static [u8],
    /// The 48-pixel PNG.
    pub(crate) large: &'static [u8],
}

/// The toolbar image `name` of `web/assets/images`, with its `_large`
/// twin.
macro_rules! toolbar_image {
    ($name:literal) => {
        ToolbarImage {
            normal: include_bytes!(concat!("../web/assets/images/", $name, ".png")),
            large: include_bytes!(concat!("../web/assets/images/", $name, "_large.png")),
        }
    };
}

/// A house: back to the original view.
pub(crate) const HOME: ToolbarImage = toolbar_image!("home");
/// A left arrow: the previous view.
pub(crate) const BACK: ToolbarImage = toolbar_image!("back");
/// A right arrow: the next view.
pub(crate) const FORWARD: ToolbarImage = toolbar_image!("forward");
/// Four arrows: pan.
pub(crate) const MOVE: ToolbarImage = toolbar_image!("move");
/// A magnifying glass over a rectangle: zoom to a rectangle.
pub(crate) const ZOOM_TO_RECT: ToolbarImage = toolbar_image!("zoom_to_rect");
/// Sliders: configure the subplots.
pub(crate) const SUBPLOTS: ToolbarImage = toolbar_image!("subplots");
/// A chart with a pen: the figure options, as matplotlib's Qt toolbar
/// shows them.
pub(crate) const FIGURE_OPTIONS: ToolbarImage = toolbar_image!("qt4_editor_options");
/// A floppy disk: save the figure.
pub(crate) const FILESAVE: ToolbarImage = toolbar_image!("filesave");
/// A question mark: the help, as matplotlib's tool manager shows it.
pub(crate) const HELP: ToolbarImage = toolbar_image!("help");

impl ToolbarImage {
    /// The image as a picture for toolbar buttons, drawn crisply at pixel
    /// ratios 1 and 2.
    pub(crate) fn picture(self) -> Picture {
        let bitmaps = [self.normal, self.large].map(|png| {
            let image = pltrs_codec::png::decode(png).expect("the bundled toolbar images decode");
            Pixmap::from_vec(image.width(), image.height(), image.to_rgba8())
                .expect("a decoded image fills a pixmap of its size")
        });
        Picture::new(SIDE, bitmaps.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_toolbar_image_comes_at_one_and_two_times() {
        let images = [
            HOME,
            BACK,
            FORWARD,
            MOVE,
            ZOOM_TO_RECT,
            SUBPLOTS,
            FIGURE_OPTIONS,
            FILESAVE,
            HELP,
        ];
        for image in images {
            for (png, side) in [(image.normal, 24), (image.large, 48)] {
                let decoded = pltrs_codec::png::decode(png).unwrap();
                assert_eq!((decoded.width(), decoded.height()), (side, side));
            }
            assert_eq!(image.picture().side(), SIDE);
        }
    }
}
