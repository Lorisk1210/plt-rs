//! Chapter 9: saving figures, and the output formats.
//!
//! [`Figure::savefig`] uses the file extension to select the format. There
//! are two families of formats:
//!
//! * Raster images are grids of pixels: PNG (`.png`), JPEG (`.jpg`), TIFF
//!   (`.tif`), GIF (`.gif`), lossless WebP (`.webp`), AVIF (`.avif`) and
//!   raw RGBA bytes (`.raw`). The size in pixels is the size in inches
//!   times the resolution. Use PNG for screens, web pages and documents.
//!   JPEG blurs thin lines and text. AVIF is smaller than PNG with a small
//!   loss, or lossless at quality 100.
//! * Vector documents stay sharp at any zoom. Use SVG (`.svg`, or
//!   compressed `.svgz`) for the web, PDF (`.pdf`) for papers and print,
//!   and PostScript (`.ps`, `.eps`) for older print workflows. PGF
//!   (`.pgf`) is a picture that LaTeX typesets with the document fonts.
//!
//! ```
//! use pltrs::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("pltrs-tutorial-saving-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # std::env::set_current_dir(&dir)?;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0, 3.0], [1.0, 3.0, 2.0, 4.0])?;
//! for name in ["plot.png", "plot.svg", "plot.pdf", "plot.eps", "plot.webp"] {
//!     fig.savefig(name)?;
//! }
//! assert!(fig.savefig("plot.docx").is_err(), "not a format pltrs writes");
//! # assert!(std::fs::read_to_string("plot.svg")?.contains("<svg"));
//! # std::env::set_current_dir(std::env::temp_dir())?; // Windows cannot remove it otherwise
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Into memory
//!
//! Each format has a method that returns the file contents: bytes for the
//! raster formats and PDF, a `String` for SVG, PostScript and PGF.
//! [`Figure::to_format`] takes a [`SaveFormat`], which parses from the
//! extensions.
//!
//! ```
//! use pltrs::figure::{SaveFormat, SaveOptions};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//!
//! let svg: String = fig.to_svg()?;
//! assert!(svg.contains("<svg"));
//! let pdf: Vec<u8> = fig.to_pdf()?;
//! assert!(pdf.starts_with(b"%PDF"));
//! let webp = fig.to_format("webp".parse::<SaveFormat>()?, SaveOptions::default())?;
//! assert!(webp.starts_with(b"RIFF"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Save options
//!
//! [`Figure::savefig_with`] and the `_with` methods for memory output take
//! a [`SaveOptions`]. A field that is `None` uses the `savefig.*` settings.
//!
//! * `dpi` is the resolution of raster output. The default is the
//!   resolution of the figure. Vector documents use points, but draw
//!   images at this resolution.
//! * `bbox_inches: Some(BboxInches::Tight)` crops or grows the output to
//!   what is drawn, plus `pad_inches`.
//! * `transparent: Some(true)` removes the figure and axes backgrounds.
//!   `facecolor` sets another background color for the figure.
//! * `metadata` holds the title, author, subject and keywords of vector
//!   documents and the text entries of PNG files. The other raster formats
//!   refuse metadata ([`SaveError::MetadataUnsupported`]).
//! * `jpeg_quality` is from 1 to 100, with 75 by default. `avif_quality` is
//!   from 0 to 100, with 75 by default and 100 for lossless.
//!
//! ```
//! use pltrs::figure::{BboxInches, Metadata, SaveOptions};
//! use pltrs::image::imread_bytes;
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//!
//! let sharp = fig.to_png_with(SaveOptions { dpi: Some(200.0), ..SaveOptions::default() })?;
//! assert_eq!(imread_bytes(&sharp)?.shape(), (800, 1200), "twice the pixels each way");
//!
//! let cropped = SaveOptions {
//!     bbox_inches: Some(BboxInches::Tight),
//!     transparent: Some(true),
//!     ..SaveOptions::default()
//! };
//! assert!(fig.to_png_with(cropped)?.starts_with(b"\x89PNG"));
//!
//! let described = SaveOptions {
//!     metadata: Some(Metadata {
//!         title: Some("Growth".into()),
//!         author: Some("A. Rustacean".into()),
//!         ..Metadata::default()
//!     }),
//!     ..SaveOptions::default()
//! };
//! let pdf = fig.to_pdf_with(described)?;
//! assert!(pdf.windows(6).any(|bytes| bytes == b"Growth"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! As in matplotlib, the settings for one format are runtime settings:
//! `svg.fonttype` (text as paths or as text), `pdf.fonttype`,
//! `ps.papersize` and others (chapter [`styling`](super::styling)).
//! Documents record the date of their creation, or the date in the
//! environment variable `SOURCE_DATE_EPOCH`. Use it for reproducible
//! files.
//!
//! # Several pages
//!
//! [`PdfPages`] collects figures as the pages of one PDF document.
//!
//! ```
//! use pltrs::figure::PdfPages;
//! use pltrs::prelude::*;
//!
//! let mut pages = PdfPages::new()?;
//! for n in 1..=3 {
//!     let fig = Figure::with_size(4.0, 3.0, 72.0);
//!     fig.subplot().plot([0.0, 1.0], [0.0, f64::from(n)])?;
//!     pages.savefig(&fig)?;
//! }
//! assert_eq!(pages.page_count(), 3);
//! let pdf = pages.finish()?; // `pages.save(path)?` writes a file instead
//! assert!(pdf.starts_with(b"%PDF"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! pltrs saves animations as animated PNG, GIF or WebP files, as an HTML
//! page, or as videos through ffmpeg (chapter
//! [`interactive`](super::interactive)). The page
//! [`porting::saving`](crate::porting::saving) lists all options.
//!
//! Next: [interactive figures](super::interactive).
//!
//! [`Figure::savefig`]: crate::figure::Figure::savefig
//! [`Figure::savefig_with`]: crate::figure::Figure::savefig_with
//! [`Figure::to_format`]: crate::figure::Figure::to_format
//! [`SaveFormat`]: crate::figure::SaveFormat
//! [`SaveOptions`]: crate::figure::SaveOptions
//! [`SaveError::MetadataUnsupported`]: crate::figure::SaveError::MetadataUnsupported
//! [`PdfPages`]: crate::figure::PdfPages
