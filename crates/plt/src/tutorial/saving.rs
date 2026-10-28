//! Chapter 9: saving figures, and the output formats.
//!
//! [`Figure::savefig`] writes a figure in the format its file name's
//! extension names. Two families of formats suit different uses:
//!
//! * **Raster images** are grids of pixels: PNG (`.png`), JPEG (`.jpg`),
//!   TIFF (`.tif`), GIF (`.gif`), lossless WebP (`.webp`) and raw RGBA
//!   bytes (`.raw`). Their size in pixels is the figure's size in inches
//!   times the resolution. PNG is the usual choice for screens, web pages
//!   and documents; JPEG blurs thin lines and text and suits photographs
//!   better than plots.
//! * **Vector documents** describe lines, shapes and text as such, and
//!   stay sharp at any zoom: SVG (`.svg`, or compressed `.svgz`) for the
//!   web, PDF (`.pdf`) for papers and print, PostScript (`.ps`, `.eps`)
//!   for older print workflows. PGF (`.pgf`) is a picture for LaTeX
//!   documents, which LaTeX typesets with the document's own fonts.
//!
//! ```
//! use plt::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("plt-tutorial-saving-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # std::env::set_current_dir(&dir)?;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! fig.subplot().plot([0.0, 1.0, 2.0, 3.0], [1.0, 3.0, 2.0, 4.0])?;
//! for name in ["plot.png", "plot.svg", "plot.pdf", "plot.eps", "plot.webp"] {
//!     fig.savefig(name)?;
//! }
//! assert!(fig.savefig("plot.docx").is_err(), "not a format plt-rs writes");
//! # assert!(std::fs::read_to_string("plot.svg")?.contains("<svg"));
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Into memory
//!
//! Each format has a method returning the file's contents instead of
//! writing it: bytes for raster formats and PDF, a `String` for the text
//! formats (SVG, PostScript, PGF). [`Figure::to_format`] takes the format
//! as a [`SaveFormat`] value, which parses the extensions.
//!
//! ```
//! use plt::figure::{SaveFormat, SaveOptions};
//! use plt::prelude::*;
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
//! [`Figure::savefig_with`] and the `_with` variants of the memory
//! methods take a [`SaveOptions`]. Every field left `None` follows the
//! `savefig.*` settings:
//!
//! * `dpi`: the resolution of raster output, by default the figure's
//!   own. Vector documents are measured in points whatever it is, but
//!   draw images at this resolution.
//! * `bbox_inches: Some(BboxInches::Tight)`: crop (or grow) the output
//!   to what is drawn, plus `pad_inches` around it, instead of saving the
//!   whole figure.
//! * `transparent: Some(true)`: leave out the figure and axes
//!   backgrounds; `facecolor` paints the figure's background in another
//!   colour.
//! * `metadata`: the title, author, subject and keywords of the vector
//!   documents, and the text entries of PNG files. The other raster
//!   formats have no room for them and refuse them
//!   ([`SaveError::MetadataUnsupported`]), so a loop saving every format
//!   gives them options without it.
//! * `jpeg_quality`: from 1 to 100, 75 by default.
//!
//! ```
//! use plt::figure::{BboxInches, Metadata, SaveOptions};
//! use plt::image::imread_bytes;
//! use plt::prelude::*;
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
//! The settings that belong to one format stay runtime settings, as in
//! matplotlib: `svg.fonttype` (text as paths or as text), `pdf.fonttype`,
//! `ps.papersize` and others (chapter [`styling`](super::styling)).
//! Documents record the date they were made, or the date in the
//! `SOURCE_DATE_EPOCH` environment variable, so that builds can produce
//! identical files.
//!
//! # Several pages
//!
//! [`PdfPages`] collects figures as the pages of one PDF document, a
//! report for instance.
//!
//! ```
//! use plt::figure::PdfPages;
//! use plt::prelude::*;
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
//! Animations save as animated PNG, GIF or WebP files, an HTML page, or
//! through ffmpeg as videos (chapter [`interactive`](super::interactive)).
//! The [`porting::saving`](crate::porting::saving) page has the full
//! list of options.
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
