//! Chapter 7: text and annotation.
//!
//! Titles, axis labels, legend texts and the texts you place yourself
//! are [`Text`] artists. The methods that add text return its handle,
//! and the handle styles it: font size, colour, weight, alignment,
//! rotation, a box behind it. A text between two `$` signs is typeset as
//! a mathematical formula, without needing a TeX installation; with one,
//! LaTeX can typeset text too.
//!
//! # Titles and labels
//!
//! [`Axes::set_title`] puts the title where the `axes.titlelocation`
//! setting says; [`Axes::set_title_at`] chooses the place (left, centre
//! or right), and an axes can have a title at each of the three. The
//! label setters chain, and
//! [`Axis::set_label_color`](crate::axes::Axis::set_label_color) and its
//! siblings style an axis label afterwards.
//!
//! ```
//! use plt::axes::TitleLocation;
//! use plt::fonts::FontWeight;
//! use plt::prelude::*;
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
//! [`Axes::text`] places a text at a point in data coordinates, so it
//! moves with the data when the limits change. To pin a text to a place
//! in the axes instead, give it the axes' own coordinate system,
//! [`Axes::trans_axes`]: there `(0, 0)` is the lower left corner of the
//! axes and `(1, 1)` the upper right, whatever the data. The alignment
//! says which point of the text sits at the position.
//!
//! ```
//! use plt::patches::{BoxStyle, PatchProps};
//! use plt::prelude::*;
//! use plt::text::{HAlign, TextBoxProps, VAlign};
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 1.0, 2.0, 3.0], [0.0, 2.0, 1.0, 3.0])?;
//!
//! // At the data point (1, 2), centred above it.
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
//! [`Figure::text`] places a text in fractions of the whole figure.
//! [`porting::text`](crate::porting::text) lists the text properties.
//!
//! # Annotations
//!
//! An annotation is a text that points at something. [`Axes::annotate`]
//! takes the text and the point `xy` it refers to; the
//! [`AnnotateOptions`](crate::text::AnnotateOptions) say where the text
//! goes (`xytext`) and how the arrow from the text to the point looks.
//! An arrow parses from matplotlib's names of arrow styles: `"->"` is a
//! line with an open head, `"-|>"` a filled head, `"<->"` heads at both
//! ends; [`FancyArrowProps`](crate::text::FancyArrowProps) holds the rest
//! of its settings.
//!
//! Positions can be in other coordinate systems than the data's, chosen
//! by a [`Coords`](crate::coords::Coords) value that parses matplotlib's
//! names. A common pair is a point in data coordinates with the text
//! offset from it by a distance in points (`"offset points"`), so the text
//! keeps its distance whatever the limits.
//!
//! ```
//! use plt::prelude::*;
//! use plt::text::AnnotateOptions;
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
//! Gallery: `annotations` in `text.rs`; `arrow_styles`,
//! `connection_styles` and `box_styles` in `annotations.rs` show every
//! style.
//!
//! # Mathematical text
//!
//! Text between `$` signs is *mathtext*: a subset of TeX's math notation
//! typeset by plt-rs itself. Greek letters (`\alpha`), sub- and
//! superscripts (`x_i^2`), fractions (`\frac{a}{b}`), roots (`\sqrt{x}`),
//! operators (`\sum`, `\int`) and accents all work, in titles, labels,
//! tick labels, legends and annotations alike. Write it in raw strings
//! (`r"..."`) so backslashes stay as they are.
//!
//! A string with an even number of unescaped `$` signs is typeset this
//! way; escape a literal dollar as `\$`, or turn math off for one text
//! with [`Text::set_parse_math`]. A malformed formula is reported when
//! the figure is drawn, as a drawing error; [`Text::layout`] checks a text
//! on its own.
//!
//! ```
//! use plt::prelude::*;
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
//! The math takes its fonts from the `mathtext.*` settings, which each
//! look sets to match its text; `mathtext.fontset` selects Computer
//! Modern (`"cm"`), STIX and others. Gallery: `mathtext` in `text.rs`.
//!
//! # Text typeset by LaTeX
//!
//! With a TeX installation (`latex`, plus `dvipng` and `kpsewhich`, as
//! matplotlib needs them), LaTeX itself can typeset the text: every text
//! made while the `text.usetex` setting is on, or one text with
//! [`Text::set_usetex`]. The whole string goes to LaTeX, so it may use any
//! LaTeX the preamble (`text.latex.preamble`) provides, and math is
//! written between `$` as in a LaTeX document. The font comes from
//! `font.family`: `"serif"` (Computer Modern), `"sans-serif"`,
//! `"monospace"`, or one of the PostScript fonts LaTeX knows
//! (`"helvetica"`, `"times"`, `"palatino"`, ...). Tick labels follow the
//! setting too.
//!
//! The typeset strings are cached in the cache directory
//! ([`config::cache_dir`](crate::config::cache_dir)), so a figure is
//! typeset once. Without TeX, drawing such a figure fails with an error
//! saying which program is missing; [`check_usetex`] tells beforehand.
//! Every format takes TeX text; PostScript output runs LaTeX's `psfrag`,
//! `dvips` and Ghostscript on the whole figure, as matplotlib does.
//!
//! ```
//! use plt::prelude::*;
//! use plt::rc;
//!
//! # rc::context(|| -> Result<(), Box<dyn std::error::Error>> {
//! rc::set("text.usetex", "True")?;
//! rc::set("font.family", "serif")?;
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! ax.set_title(r"\textbf{Growth} of $x^2$");
//! if plt::text::check_usetex().is_ok() {
//!     assert!(!fig.to_png()?.is_empty());
//! }
//! # Ok(())
//! # })?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Fonts
//!
//! The `font.family` and `font.size` settings choose the default font
//! (chapter [`styling`](super::styling)); a text's own family, size,
//! weight and style override them ([`Text::set_family`],
//! [`Text::set_fontsize`], [`Text::set_weight`], [`Text::set_style`]).
//! plt-rs bundles `DejaVu Sans`, `DejaVu Serif`, Inter and the mathtext
//! fonts, and finds fonts installed on the system by family name.
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
