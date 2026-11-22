//! Text, fonts, mathtext, text that LaTeX typesets, and annotations.
//!
//! [`Axes::text`], [`Figure::text`] and the title and label setters return
//! a [`Text`] handle. In place of the keywords (`fontsize=`, `ha=`,
//! `bbox=`, ...), style the handle, or apply a [`TextProps`] with
//! [`Text::update_props`].
//! Alignments are enums ([`HAlign`], [`VAlign`]) that parse the matplotlib
//! names. A text box is a [`TextBoxProps`]. Its [`BoxStyle`] parses the
//! matplotlib strings, such as `"round,pad=0.3"`.
//!
//! Fonts work as in the matplotlib font manager. A [`FontProperties`]
//! describes the font that you want: families, style, weight, stretch and
//! size. The [`FontManager`] finds the file. A size is points or a
//! [`RelativeSize`] keyword ([`FontSize`]). Weights and stretches are
//! numeric newtypes with named constants ([`FontWeight::BOLD`]).
//!
//! Mathtext works as in matplotlib. pltrs typesets each line with an even
//! number of unescaped `$`. It uses the fontset of `mathtext.fontset`, or
//! the fontset of the text ([`Text::set_math_fontfamily`]). Errors in the
//! math show when pltrs lays out the text ([`Text::layout`]) or draws it.
//!
//! `text.usetex` works as in matplotlib. `latex` typesets a text if
//! `text.usetex` is on when you make it, or after
//! [`Text::set_usetex`]`(true)`. It uses the fonts of `font.family`, the
//! family lists, and `text.latex.preamble`. pltrs caches the results in
//! `tex.cache` under the [cache directory](crate::config::cache_dir).
//! Tick formatters put their labels in `$\mathdefault{...}$`.
//!
//! * PNG draws the glyphs of the DVI file, or the image of `dvipng` with
//!   `text.latex.engine = "latex+dvipng"`.
//! * PDF embeds the Type 1 fonts. SVG draws glyph paths. PGF gives the
//!   string to LaTeX.
//! * PostScript and EPS tag the texts for the LaTeX package `psfrag`.
//!   Then LaTeX, `dvips` and Ghostscript run on the whole figure.
//!
//! If a program is missing, the draw or save fails with the matplotlib
//! message. [`check_usetex`] and [`check_usetex_postscript`] check the
//! programs before. A LaTeX error fails with the output of `latex`.
//!
//! Annotations take their keywords in an [`AnnotateOptions`]. Coordinate
//! systems are [`Coords`] values, which parse the matplotlib names
//! (`"axes fraction"`, `"offset points"`, ...). The arrow is an
//! [`ArrowProps`].
//!
//! # Equivalences
//!
//! | matplotlib                                    | pltrs                                             |
//! |-----------------------------------------------|---------------------------------------------------|
//! | `ax.text(x, y, s)`, `fig.text(x, y, s)`       | [`Axes::text`], [`Figure::text`]                  |
//! | `text(..., transform=ax.transAxes)`           | [`ArtistHandle::set_transform`]`(ax.trans_axes())` |
//! | `t.set_text(s)`, `t.get_text()`               | [`Text::set_text`], [`Text::text`]                |
//! | `fontsize=12`, `fontsize="large"`             | [`Text::set_fontsize`]`(12.0)`, [`Text::set_fontsize`]`(RelativeSize::Large)` |
//! | `color=`, `rotation=`, `rotation_mode=`       | [`Text::set_color`], [`Text::set_rotation`], [`Text::set_rotation_mode`] |
//! | `ha="center"`, `va="top"`, `multialignment=`  | [`Text::set_halign`], [`Text::set_valign`], [`Text::set_multialign`], and `"top".parse::<VAlign>()?` |
//! | `fontfamily=`, `fontstyle=`, `fontweight=`    | [`Text::set_family`], [`Text::set_style`], [`Text::set_weight`] |
//! | `fontproperties=fp`, `t.get_fontproperties()` | [`Text::set_font`], [`Text::font`]                |
//! | `linespacing=`, `wrap=True`                   | [`Text::set_linespacing`], [`Text::set_wrap`]     |
//! | `bbox=dict(boxstyle="round", fc="w")`         | [`Text::set_bbox`]`(Some(TextBoxProps::new(..).with_patch(..)))` |
//! | `backgroundcolor=`                            | [`Text::set_backgroundcolor`]                     |
//! | `parse_math=False`                            | [`Text::set_parse_math`]`(false)`                 |
//! | `usetex=True`, `t.set_usetex(None)`           | [`Text::set_usetex`]`(true)`, [`Text::set_usetex`]`(rc::params().text_usetex)` |
//! | `rcParams["text.usetex"] = True`              | `rc::set("text.usetex", "True")`                  |
//! | `rcParams["text.latex.preamble"] = r"\usepackage{amsmath}"` | `rc::set("text.latex.preamble", r"\usepackage{amsmath}")` |
//! | `matplotlib.checkdep_usetex(True)`            | [`check_usetex`]                                  |
//! | `TextPath(xy, s, usetex=True)`                | [`text_path_tex`]                                 |
//! | `math_fontfamily="cm"`                        | [`Text::set_math_fontfamily`]`(Fontset::Cm)` ([`Fontset`]) |
//! | `t.set(**kwargs)`                             | [`Text::update_props`] with [`TextProps`]         |
//! | `t.get_window_extent()`                       | [`ArtistHandle::window_extent`]                   |
//! | `ax.set_title(s, loc="left", pad=10)`         | [`Axes::set_title_at`]`(TitleLocation::Left, s)`, [`Axes::set_title_pad`] |
//! | `fig.suptitle`, `fig.supxlabel`               | [`Figure::suptitle`], [`Figure::supxlabel`]       |
//! | `FontProperties(family=, style=, weight=, size=)` | [`FontProperties`] (public fields)            |
//! | `font_manager.findfont(fp)`                   | [`FontManager::find_font`]                        |
//! | `font_manager.fontManager.addfont(path)`      | [`FontManager::add_font`]                         |
//! | `rcParams["font.family"] = "serif"`           | `rc::set("font.family", "serif")` ([`crate::rc::set`]) |
//! | `rcParams["mathtext.fontset"] = "stix"`       | `rc::set("mathtext.fontset", "stix")`             |
//! | `TextPath`                                    | [`text_path`], [`measure_text`]                   |
//! | `mathtext.math_to_image(s, file, prop=, dpi=)` | [`math_to_image`] with [`MathImageOptions`]       |
//! | `ax.annotate(s, xy, xytext=, arrowprops=)`    | [`Axes::annotate`] with [`AnnotateOptions`]       |
//! | `xycoords="axes fraction"`, `textcoords="offset points"` | `"axes fraction".parse::<Coords>()?` ([`Coords`]) |
//! | `xycoords=artist`, `xycoords=(x_coords, y_coords)` | [`Coords::Artist`], [`Coords::blended`]      |
//! | `arrowprops=dict(arrowstyle="->", connectionstyle="arc3,rad=.2")` | [`FancyArrowProps`], [`ConnectionStyle`] parsed from the same strings |
//! | `arrowprops=dict(width=, headwidth=, shrink=)` | [`ArrowProps::Legacy`] with [`LegacyArrowProps`] |
//! | `ann.xy`, `ann.set_anncoords(...)`            | [`Annotation::xy`], [`Annotation::set_textcoords`] |
//! | `ann.draggable()`                             | [`Annotation::set_draggable`]                     |
//! | `AnchoredText(s, loc=)`                       | [`AnchoredBox`] with a [`TextArea`] ([`Axes::add_anchored_box`]) |
//! | `TexManager()` and its `make_dvi`, `make_png`, `get_grey` | [`tex_manager`]`(&rc::params())?` and [`TexManager::make_dvi`], [`TexManager::make_png`], [`TexManager::grey`] |
//! | `TexManager().get_rgba(tex, fontsize, dpi, rgb)` | [`TexManager::rgba`]                            |
//! | `dviread.Dvi(path, dpi)`, its pages           | `pltrs_text::tex::read_dvi`, [`TexFiles::dvi_pages`] ([`DviPage`]: `text`, `boxes`) |
//! | `PsfontsMap(find_tex_file("pdftex.map"))[name]` | [`PsfontsMap::parse`], [`PsfontsMap::get`], [`FileFinder`] |
//! | `ft2font.FT2Font(path)`, `set_size`, `set_text`, `load_char` | `pltrs_font` (a separate crate): `Face::parse`, `Size::new`, `FontChain::layout`, `Face::glyph_index` with a `Hinter` |
//! | `FT2Font.get_sfnt_table("head")`, `get_kerning` | `Face::head`, `Face::os2`, ..., `Size::kerning` |
//!
//! # Styling a text
//!
//! ```python
//! t = ax.text(0.5, 0.5, "centred", transform=ax.transAxes,
//!             ha="center", va="center", fontsize=14, color="tab:red",
//!             rotation=15, fontweight="bold",
//!             bbox=dict(boxstyle="round,pad=0.3", facecolor="w"))
//! t.set_text("changed")
//! ```
//!
//! ```
//! use pltrs::fonts::FontWeight;
//! use pltrs::patches::{BoxStyle, PatchProps};
//! use pltrs::prelude::*;
//! use pltrs::text::{HAlign, TextBoxProps, VAlign};
//!
//! let ax = Figure::new().subplot();
//! let t = ax.text(0.5, 0.5, "centred");
//! t.set_transform(ax.trans_axes())
//!     .set_halign(HAlign::Center)
//!     .set_valign(VAlign::Center)
//!     .set_fontsize(14.0)
//!     .set_color("tab:red")
//!     .set_rotation(15.0)
//!     .set_weight(FontWeight::BOLD);
//! let frame = TextBoxProps::new("round,pad=0.3".parse::<BoxStyle>()?)
//!     .with_patch(PatchProps::default().facecolor("w"));
//! t.set_bbox(Some(frame));
//! t.set_text("changed");
//!
//! assert_eq!(t.text(), "changed");
//! assert_eq!(t.font().size, 14.0);
//! assert_eq!(t.font().weight, FontWeight::BOLD);
//! assert!(t.has_bbox());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Fonts
//!
//! ```python
//! t.set_fontfamily(["DejaVu Serif"])
//! t.set_fontstyle("italic")
//! t.set_fontsize("large")
//! fp = FontProperties(family="DejaVu Sans", weight="bold")
//! assert findfont(fp).endswith("DejaVuSans-Bold.ttf")
//! ```
//!
//! ```
//! use pltrs::fonts::{FontExt, FontManager, FontProperties, FontStyle, FontWeight, RelativeSize};
//! use pltrs::prelude::*;
//! use pltrs::rc;
//!
//! let ax = Figure::new().subplot();
//! let t = ax.text(0.0, 0.0, "styled");
//! t.set_family(["DejaVu Serif"])
//!     .set_style(FontStyle::Italic)
//!     .set_fontsize(RelativeSize::Large);
//! assert_eq!(t.font().families, ["DejaVu Serif"]);
//!
//! let params = rc::params();
//! let wanted = FontProperties {
//!     families: vec!["DejaVu Sans".into()],
//!     weight: FontWeight::BOLD,
//!     ..FontProperties::from_rc(&params)
//! };
//! let file = FontManager::bundled().find_font(&wanted, &params, FontExt::Ttf, true)?;
//! assert!(file.ends_with("DejaVuSans-Bold.ttf"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Mathtext
//!
//! ```python
//! t = ax.text(0, 0, r"$\alpha^2 + \beta_i$", math_fontfamily="cm")
//! plain = ax.text(0, 1, r"costs $5 or $6", parse_math=False)
//! ax.text(0, 2, r"$\frac{1$")      # ValueError when the figure is drawn
//! ```
//!
//! ```
//! use pltrs::fonts::Fontset;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let t = ax.text(0.0, 0.0, r"$\alpha^2 + \beta_i$");
//! t.set_math_fontfamily(Fontset::Cm);
//! assert!(t.layout().is_ok());
//!
//! let plain = ax.text(0.0, 1.0, "costs $5 or $6");
//! plain.set_parse_math(false);
//! assert!(plain.layout().is_ok());
//!
//! let broken = ax.text(0.0, 2.0, r"$\frac{1$");
//! assert!(broken.layout().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # TeX
//!
//! ```python
//! plt.rcParams["text.latex.preamble"] = r"\usepackage{amssymb}"
//! t = ax.text(0, 0, r"$\mathbb{R}^2$, \LaTeX", usetex=True)
//! fig.savefig("tex.pdf")    # RuntimeError where TeX is not installed
//! ```
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::rc;
//!
//! # rc::context(|| -> Result<(), Box<dyn std::error::Error>> {
//! rc::set("text.latex.preamble", r"\usepackage{amssymb}")?;
//! let fig = Figure::new();
//! let t = fig.subplot().text(0.0, 0.0, r"$\mathbb{R}^2$, \LaTeX");
//! t.set_usetex(true);
//! assert!(t.usetex());
//! if pltrs::text::check_usetex().is_ok() {
//!     assert!(fig.to_pdf().is_ok());
//! }
//! # Ok(())
//! # })?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Annotations
//!
//! ```python
//! ax.annotate("max", xy=(2, 4), xytext=(3, 5),
//!             arrowprops=dict(arrowstyle="->", connectionstyle="arc3,rad=0.2"))
//! corner = ax.annotate("corner", xy=(1, 1), xycoords="axes fraction",
//!                      xytext=(-10, -10), textcoords="offset points",
//!                      ha="right", va="top")
//! ```
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::text::{AnnotateOptions, FancyArrowProps, HAlign, VAlign};
//! use pltrs::transforms::Point;
//!
//! let ax = Figure::new().subplot();
//! let mut arrow: FancyArrowProps = "->".parse()?;
//! arrow.connectionstyle = "arc3,rad=0.2".parse()?;
//! let options = AnnotateOptions {
//!     xytext: Some((3.0, 5.0).into()),
//!     arrowprops: Some(arrow.into()),
//!     ..AnnotateOptions::default()
//! };
//! let peak = ax.annotate("max", (2.0, 4.0), options)?;
//! assert_eq!(peak.xy(), Point::new(2.0, 4.0));
//!
//! let options = AnnotateOptions {
//!     xycoords: "axes fraction".parse()?,
//!     xytext: Some((-10.0, -10.0).into()),
//!     textcoords: Some("offset points".parse()?),
//!     ..AnnotateOptions::default()
//! };
//! let corner = ax.annotate("corner", (1.0, 1.0), options)?;
//! corner.set_halign(HAlign::Right).set_valign(VAlign::Top);
//! assert!("axes inches".parse::<pltrs::coords::Coords>().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes::text`]: crate::axes::Axes::text
//! [`Axes::set_title_at`]: crate::axes::Axes::set_title_at
//! [`Axes::set_title_pad`]: crate::axes::Axes::set_title_pad
//! [`Axes::annotate`]: crate::axes::Axes::annotate
//! [`Axes::add_anchored_box`]: crate::axes::Axes::add_anchored_box
//! [`Figure::text`]: crate::figure::Figure::text
//! [`Figure::suptitle`]: crate::figure::Figure::suptitle
//! [`Figure::supxlabel`]: crate::figure::Figure::supxlabel
//! [`ArtistHandle::set_transform`]: crate::artist::ArtistHandle::set_transform
//! [`ArtistHandle::window_extent`]: crate::artist::ArtistHandle::window_extent
//! [`Text`]: crate::text::Text
//! [`Text::update_props`]: crate::text::Text::update_props
//! [`Text::set_text`]: crate::text::Text::set_text
//! [`Text::text`]: crate::text::Text::text
//! [`Text::set_fontsize`]: crate::text::Text::set_fontsize
//! [`Text::set_color`]: crate::text::Text::set_color
//! [`Text::set_rotation`]: crate::text::Text::set_rotation
//! [`Text::set_rotation_mode`]: crate::text::Text::set_rotation_mode
//! [`Text::set_halign`]: crate::text::Text::set_halign
//! [`Text::set_valign`]: crate::text::Text::set_valign
//! [`Text::set_multialign`]: crate::text::Text::set_multialign
//! [`Text::set_family`]: crate::text::Text::set_family
//! [`Text::set_style`]: crate::text::Text::set_style
//! [`Text::set_weight`]: crate::text::Text::set_weight
//! [`Text::set_font`]: crate::text::Text::set_font
//! [`Text::font`]: crate::text::Text::font
//! [`Text::set_linespacing`]: crate::text::Text::set_linespacing
//! [`Text::set_wrap`]: crate::text::Text::set_wrap
//! [`Text::set_bbox`]: crate::text::Text::set_bbox
//! [`Text::set_backgroundcolor`]: crate::text::Text::set_backgroundcolor
//! [`Text::set_parse_math`]: crate::text::Text::set_parse_math
//! [`Text::set_math_fontfamily`]: crate::text::Text::set_math_fontfamily
//! [`Text::set_usetex`]: crate::text::Text::set_usetex
//! [`check_usetex`]: crate::text::check_usetex
//! [`check_usetex_postscript`]: crate::text::check_usetex_postscript
//! [`text_path_tex`]: crate::text::text_path_tex
//! [`Text::layout`]: crate::text::Text::layout
//! [`TextProps`]: crate::text::TextProps
//! [`TextBoxProps`]: crate::text::TextBoxProps
//! [`HAlign`]: crate::text::HAlign
//! [`VAlign`]: crate::text::VAlign
//! [`AnnotateOptions`]: crate::text::AnnotateOptions
//! [`ArrowProps`]: crate::text::ArrowProps
//! [`ArrowProps::Legacy`]: crate::text::ArrowProps::Legacy
//! [`LegacyArrowProps`]: crate::text::LegacyArrowProps
//! [`FancyArrowProps`]: crate::text::FancyArrowProps
//! [`Annotation::xy`]: crate::text::Annotation::xy
//! [`Annotation::set_textcoords`]: crate::text::Annotation::set_textcoords
//! [`Annotation::set_draggable`]: crate::text::Annotation::set_draggable
//! [`text_path`]: crate::text::text_path
//! [`measure_text`]: crate::text::measure_text
//! [`math_to_image`]: crate::text::math_to_image
//! [`MathImageOptions`]: crate::text::MathImageOptions
//! [`BoxStyle`]: crate::patches::BoxStyle
//! [`ConnectionStyle`]: crate::patches::ConnectionStyle
//! [`Coords`]: crate::coords::Coords
//! [`Coords::Artist`]: crate::coords::Coords::Artist
//! [`Coords::blended`]: crate::coords::Coords::blended
//! [`FontProperties`]: crate::fonts::FontProperties
//! [`FontManager`]: crate::fonts::FontManager
//! [`FontManager::find_font`]: crate::fonts::FontManager::find_font
//! [`FontManager::add_font`]: crate::fonts::FontManager::add_font
//! [`RelativeSize`]: crate::fonts::RelativeSize
//! [`FontSize`]: crate::fonts::FontSize
//! [`FontWeight::BOLD`]: crate::fonts::FontWeight::BOLD
//! [`Fontset`]: crate::fonts::Fontset
//! [`AnchoredBox`]: crate::offsetbox::AnchoredBox
//! [`TextArea`]: crate::offsetbox::TextArea
//! [`tex_manager`]: crate::text::tex_manager
//! [`TexManager::make_dvi`]: pltrs_text::tex::TexManager::make_dvi
//! [`TexManager::make_png`]: pltrs_text::tex::TexManager::make_png
//! [`TexManager::grey`]: pltrs_text::tex::TexManager::grey
//! [`TexManager::rgba`]: pltrs_text::tex::TexManager::rgba
//! [`TexFiles::dvi_pages`]: pltrs_text::tex::TexFiles::dvi_pages
//! [`DviPage`]: pltrs_text::tex::DviPage
//! [`PsfontsMap::parse`]: pltrs_text::tex::PsfontsMap::parse
//! [`PsfontsMap::get`]: pltrs_text::tex::PsfontsMap::get
//! [`FileFinder`]: pltrs_text::tex::FileFinder
