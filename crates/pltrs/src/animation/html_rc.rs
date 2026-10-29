//! The rcParams of animations as web pages: what a notebook shows
//! (`animation.html`) and the format of the player's frames
//! (`animation.frame_format`).

use crate::figure::SaveFormat;
use crate::rc::keyword_names;

/// What an animation shows in a notebook (`animation.html`, see
/// [`FuncAnimation::repr_html`](super::FuncAnimation::repr_html)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HtmlRepr {
    /// `html5`: an HTML5 video.
    Html5,
    /// `jshtml`: matplotlib's JavaScript player.
    JsHtml,
    /// `none`: nothing.
    #[default]
    None,
}

impl HtmlRepr {
    /// matplotlib's name (`html5`, `jshtml` or `none`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Html5 => "html5",
            Self::JsHtml => "jshtml",
            Self::None => "none",
        }
    }
}

keyword_names!(
    HtmlRepr,
    "HTML representation",
    [Self::Html5, Self::JsHtml, Self::None]
);

/// The format of the frames of an animation as a web page
/// (`animation.frame_format`): matplotlib's names, of which the player
/// shows SVG and PNG frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FrameFormat {
    /// `png`.
    #[default]
    Png,
    /// `jpeg`.
    Jpeg,
    /// `tiff`.
    Tiff,
    /// `raw`.
    Raw,
    /// `rgba`.
    Rgba,
    /// `ppm`.
    Ppm,
    /// `sgi`.
    Sgi,
    /// `bmp`.
    Bmp,
    /// `pbm`.
    Pbm,
    /// `svg`.
    Svg,
}

impl FrameFormat {
    const ALL: [Self; 10] = [
        Self::Png,
        Self::Jpeg,
        Self::Tiff,
        Self::Raw,
        Self::Rgba,
        Self::Ppm,
        Self::Sgi,
        Self::Bmp,
        Self::Pbm,
        Self::Svg,
    ];

    /// matplotlib's name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Tiff => "tiff",
            Self::Raw => "raw",
            Self::Rgba => "rgba",
            Self::Ppm => "ppm",
            Self::Sgi => "sgi",
            Self::Bmp => "bmp",
            Self::Pbm => "pbm",
            Self::Svg => "svg",
        }
    }

    /// The format the player's frames are saved in: SVG, or PNG for every
    /// other format (the player's JPEG and TIFF included).
    pub const fn save_format(self) -> SaveFormat {
        match self {
            Self::Svg => SaveFormat::Svg,
            _ => SaveFormat::Png,
        }
    }
}

keyword_names!(FrameFormat, "frame format", Self::ALL);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn representations_round_trip_their_names() {
        for repr in [HtmlRepr::Html5, HtmlRepr::JsHtml, HtmlRepr::None] {
            assert_eq!(repr.to_string().parse(), Ok(repr));
        }
        assert!("video".parse::<HtmlRepr>().is_err());
    }

    #[test]
    fn frames_are_svg_or_png() {
        for format in FrameFormat::ALL {
            assert_eq!(format.to_string().parse(), Ok(format));
        }
        assert_eq!(FrameFormat::Svg.save_format(), SaveFormat::Svg);
        assert_eq!(FrameFormat::Jpeg.save_format(), SaveFormat::Png);
        assert!("gif".parse::<FrameFormat>().is_err());
    }
}
