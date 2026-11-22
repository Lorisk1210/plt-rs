//! Chapter 7: text and annotation.
//!
//! All texts are [`Text`] artists. The methods that add text return its
//! handle, which sets the font, color, alignment, rotation and a box behind
//! the text. pltrs typesets text between `$` signs as a formula. If TeX is
//! installed, LaTeX can also typeset text.
//!
//! # Titles and labels
//!
//! [`Axes::set_title`] puts the title at the place that
//! `axes.titlelocation` gives. [`Axes::set_title_at`] selects the place
//! (left, center or right), and an axes can have a title at each place.
//! [`Axis::set_label_color`](crate::axes::Axis::set_label_color) and
//! similar methods style an axis label.
//!
//! ```
//! use pltrs::axes::TitleLocation;
//! use pltrs::fonts::FontWeight;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let title = ax.set_title("Monthly rainfall");
//! title.set_fontsize(16.0).set_weight(FontWeight::BOLD);
//! ax.set_title_at(TitleLocation::Right, "2024");
//! ax.set_xlabel("month").set_ylabel("rain (mm)");
//!
//! assert_eq!(ax.title(TitleLocation::Right), "2024");
//! assert_eq!(title.font().size, 16.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Text anywhere
//!
//! [`Axes::text`] places a text at a point in data coordinates. To fix a
//! text in the axes, give it the transform [`Axes::trans_axes`], where
//! `(0, 0)` is the lower left corner and `(1, 1)` the upper right corner.
//! The alignment selects the point of the text that is at the position.
//!
//! ```
//! use pltrs::patches::{BoxStyle, PatchProps};
//! use pltrs::prelude::*;
//! use pltrs::text::{HAlign, TextBoxProps, VAlign};
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 1.0, 2.0, 3.0], [0.0, 2.0, 1.0, 3.0])?;
//!
//! // At the data point (1, 2), centered above it.
//! let peak = ax.text(1.0, 2.0, "local maximum");
//! peak.set_halign(HAlign::Center).set_valign(VAlign::Bottom);
//!
//! // In the upper left corner of the axes, in a rounded box.
//! let note = ax.text(0.03, 0.95, "n = 4");
//! note.set_transform(ax.trans_axes())
//!     .set_halign(HAlign::Left)
//!     .set_valign(VAlign::Top)
//!     .set_color("0.3");
//! let frame = TextBoxProps::new("round,pad=0.3".parse::<BoxStyle>()?)
//!     .with_patch(PatchProps::default().facecolor("white").edgecolor("0.7"));
//! note.set_bbox(Some(frame));
//!
//! assert_eq!(ax.texts().len(), 2);
//! assert!(note.has_bbox());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::text`] places a text in fractions of the figure.
//! [`porting::text`](crate::porting::text) lists the text properties.
//!
//! # Annotations
//!
//! An annotation is a text that points at something. [`Axes::annotate`]
//! takes the text and the point `xy`. The
//! [`AnnotateOptions`](crate::text::AnnotateOptions) give the text
//! position (`xytext`) and the arrow. Arrow styles parse from the
//! matplotlib names: `"->"` (open head), `"-|>"` (filled head), `"<->"`
//! (two heads). [`FancyArrowProps`](crate::text::FancyArrowProps) holds
//! the other arrow settings.
//!
//! A [`Coords`](crate::coords::Coords) value, parsed from the matplotlib
//! names, selects the coordinate system of a position. Often the point is
//! in data coordinates and the text is at an offset in points
//! (`"offset points"`).
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::text::AnnotateOptions;
//!
//! let ax = Figure::new().subplot();
//! let x = linspace(0.0, 6.0, 100);
//! ax.plot(&x, x.iter().map(|x| x.sin()))?;
//!
//! let peak = std::f64::consts::FRAC_PI_2;
//! let options = AnnotateOptions {
//!     xytext: Some((40.0, -30.0).into()),
//!     textcoords: Some("offset points".parse()?),
//!     arrowprops: Some("->".parse()?),
//!     ..AnnotateOptions::default()
//! };
//! let note = ax.annotate("maximum", (peak, 1.0), options)?;
//! assert_eq!(note.text(), "maximum");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `annotations` in `text.rs`, and `arrow_styles`,
//! `connection_styles` and `box_styles` in `annotations.rs`.
//!
//! # Mathematical text
//!
//! Text between `$` signs is *mathtext*, a subset of the TeX math notation.
//! Greek letters (`\alpha`), subscripts and superscripts (`x_i^2`),
//! fractions (`\frac{a}{b}`), roots (`\sqrt{x}`), operators (`\sum`,
//! `\int`) and accents work in all texts. Use raw strings (`r"..."`).
//!
//! Math needs an even number of unescaped `$` signs. Write a literal dollar
//! as `\$`, or turn math off with [`Text::set_parse_math`]. A bad formula
//! makes the drawing fail. [`Text::layout`] checks one text.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! ax.set_title(r"$f(x) = e^{-x^2/2\sigma^2}$, $\sigma = 0.5$");
//! ax.set_xlabel(r"$x$ (m)").set_ylabel(r"$\frac{dE}{dx}$");
//!
//! let price = ax.text(0.0, 0.0, "costs $5 or $6");
//! price.set_parse_math(false);
//! assert!(price.layout().is_ok());
//!
//! let broken = ax.text(0.0, 1.0, r"$\frac{1$");
//! assert!(broken.layout().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Each look sets the `mathtext.*` fonts to match its text.
//! `mathtext.fontset` selects Computer Modern (`"cm"`), STIX and others. Gallery: `mathtext` in `text.rs`.
//!
//! # Text typeset by LaTeX
//!
//! If TeX is installed (`latex`, `dvipng` and `kpsewhich`), LaTeX can
//! typeset text. pltrs uses LaTeX for all texts made while `text.usetex`
//! is on, tick labels included, or for one text with
//! [`Text::set_usetex`]. LaTeX gets the whole string, so it can use the
//! preamble (`text.latex.preamble`), and math goes between `$` signs.
//! `font.family` selects the font: `"serif"` (Computer Modern),
//! `"sans-serif"`, `"monospace"`, or a PostScript font such as
//! `"helvetica"`, `"times"` or `"palatino"`.
//!
//! pltrs caches the typeset strings in
//! [`config::cache_dir`](crate::config::cache_dir). Without TeX, drawing
//! fails with an error that names the missing program. [`check_usetex`]
//! checks before. All formats accept LaTeX text. For PostScript, pltrs runs
//! `psfrag`, `dvips` and Ghostscript on the whole figure, as matplotlib
//! does.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::rc;
//!
//! # rc::context(|| -> Result<(), Box<dyn std::error::Error>> {
//! rc::set("text.usetex", "True")?;
//! rc::set("font.family", "serif")?;
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! ax.set_title(r"\textbf{Growth} of $x^2$");
//! if pltrs::text::check_usetex().is_ok() {
//!     assert!(!fig.to_png()?.is_empty());
//! }
//! # Ok(())
//! # })?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Fonts
//!
//! The settings `font.family` and `font.size` give the default font
//! (chapter [`styling`](super::styling)). [`Text::set_family`],
//! [`Text::set_fontsize`], [`Text::set_weight`] and [`Text::set_style`]
//! override them for one text. pltrs includes `DejaVu Sans`,
//! `DejaVu Serif`, Inter and the mathtext fonts. It finds system fonts by
//! family name.
//!
//! Next: [scales, ticks and dates](super::scales).
//!
//! [`Text`]: crate::text::Text
//! [`Text::set_parse_math`]: crate::text::Text::set_parse_math
//! [`Text::set_usetex`]: crate::text::Text::set_usetex
//! [`check_usetex`]: crate::text::check_usetex
//! [`Text::layout`]: crate::text::Text::layout
//! [`Text::set_family`]: crate::text::Text::set_family
//! [`Text::set_fontsize`]: crate::text::Text::set_fontsize
//! [`Text::set_weight`]: crate::text::Text::set_weight
//! [`Text::set_style`]: crate::text::Text::set_style
//! [`Axes::set_title`]: crate::axes::Axes::set_title
//! [`Axes::set_title_at`]: crate::axes::Axes::set_title_at
//! [`Axes::text`]: crate::axes::Axes::text
//! [`Axes::trans_axes`]: crate::axes::Axes::trans_axes
//! [`Axes::annotate`]: crate::axes::Axes::annotate
//! [`Figure::text`]: crate::figure::Figure::text
