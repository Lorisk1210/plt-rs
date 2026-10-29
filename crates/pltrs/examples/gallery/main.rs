//! A gallery of pltrs figures, one function per figure, grouped by topic
//! like matplotlib's gallery.
//!
//! ```text
//! cargo run -p pltrs --release --example gallery -- OUTDIR
//! ```
//!
//! Every figure is written to `OUTDIR/<name>.png` (`target/gallery` by
//! default), with an `index.html` showing them section by section and a
//! contact sheet of each section, `sheet-<section>.png`. Animations are
//! written as animated PNGs, which browsers play and other viewers show
//! as their first frame. The figures take the look in force
//! (see `pltrs::look`): `PLTRS_FIGURE_LOOK=modern-dark` draws the whole
//! gallery on a dark background, `PLTRS_LOOK=matplotlib` in matplotlib's
//! look.

#![expect(clippy::print_stdout, reason = "the example reports what it wrote")]

mod annotations;
mod bars;
mod colour;
mod contours;
mod data;
mod density;
mod diagrams;
mod grids;
mod images;
mod layout;
mod lines;
mod projections;
mod reference;
mod sheet;
mod spatial;
mod stats;
mod styles;
mod text;
mod three_d;
mod units;
mod vectors;

use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pltrs::animation::FuncAnimation;
use pltrs::colors::Color;
use pltrs::figure::Figure;
use pltrs::{look, rc};

/// The result of building one figure.
type Built = Result<Figure, Box<dyn Error>>;

/// The result of building one animation.
type Animated = Result<FuncAnimation, Box<dyn Error>>;

/// The face colour of axes in the look in force: edges and halos drawn
/// in it set marks apart on light and dark backgrounds alike.
fn background() -> Color {
    rc::params().axes_facecolor.clone()
}

/// How an entry of the gallery is made.
#[derive(Clone, Copy)]
enum Build {
    /// A still figure.
    Figure(fn() -> Built),
    /// An animation, saved with all its frames.
    Animation(fn() -> Animated),
}

/// One figure of the gallery: its file name, its caption and how it is
/// built.
struct Entry {
    name: &'static str,
    caption: &'static str,
    build: Build,
}

impl Entry {
    /// A still figure made by `build`.
    const fn figure(name: &'static str, caption: &'static str, build: fn() -> Built) -> Self {
        Self {
            name,
            caption,
            build: Build::Figure(build),
        }
    }

    /// An animation made by `build`.
    const fn animation(name: &'static str, caption: &'static str, build: fn() -> Animated) -> Self {
        Self {
            name,
            caption,
            build: Build::Animation(build),
        }
    }

    /// Builds the entry and writes it to `dir`, returning the file name.
    fn write(&self, dir: &Path) -> Result<String, Box<dyn Error>> {
        let file = format!("{}.png", self.name);
        match self.build {
            Build::Figure(build) => build()?.savefig(dir.join(&file))?,
            Build::Animation(build) => build()?.save_apng(dir.join(&file))?,
        }
        Ok(file)
    }
}

/// A section of the gallery: an anchor, a heading and its figures.
struct Section {
    id: &'static str,
    title: &'static str,
    entries: &'static [&'static [Entry]],
}

/// Every section, in the order of the index.
const SECTIONS: &[Section] = &[
    Section {
        id: "lines",
        title: "Lines, bars and areas",
        entries: &[lines::ENTRIES, bars::ENTRIES],
    },
    Section {
        id: "statistics",
        title: "Statistics",
        entries: &[stats::ENTRIES, density::ENTRIES],
    },
    Section {
        id: "images",
        title: "Images and meshes",
        entries: &[images::ENTRIES],
    },
    Section {
        id: "fields",
        title: "Contours and vector fields",
        entries: &[contours::ENTRIES, vectors::ENTRIES],
    },
    Section {
        id: "colour",
        title: "Colour",
        entries: &[colour::ENTRIES],
    },
    Section {
        id: "text",
        title: "Text and annotations",
        entries: &[text::ENTRIES, annotations::ENTRIES],
    },
    Section {
        id: "layout",
        title: "Layout",
        entries: &[layout::ENTRIES],
    },
    Section {
        id: "units",
        title: "Dates, categories and units",
        entries: &[units::ENTRIES],
    },
    Section {
        id: "three-d",
        title: "3-D",
        entries: &[three_d::ENTRIES],
    },
    Section {
        id: "projections",
        title: "Polar, geographic and custom projections",
        entries: &[projections::ENTRIES],
    },
    Section {
        id: "toolkits",
        title: "Toolkits: axes_grid1 and axisartist",
        entries: &[grids::ENTRIES, spatial::ENTRIES],
    },
    Section {
        id: "diagrams",
        title: "Diagrams, widgets and animation",
        entries: &[diagrams::ENTRIES],
    },
    Section {
        id: "reference",
        title: "Reference: markers, lines and hatches",
        entries: &[reference::ENTRIES],
    },
    Section {
        id: "styles",
        title: "Style sheets",
        entries: &[styles::ENTRIES],
    },
];

/// The head of the index page: its title and its style sheet.
const HEAD: &str = concat!(
    "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n",
    "<title>pltrs gallery</title>\n<style>\n",
    "body{font-family:system-ui,sans-serif;margin:2em auto;max-width:1400px;padding:0 1em}\n",
    "nav a{margin-right:1em;white-space:nowrap}\n",
    "h2{border-bottom:1px solid #ddd;padding-bottom:.2em;margin-top:2em}\n",
    "figure{display:inline-block;margin:0 1em 1.5em 0;width:320px;vertical-align:top}\n",
    "img{width:100%;border:1px solid #eee}figcaption{font-size:.9em;color:#444}\n",
    "</style></head><body>\n",
);

fn main() -> Result<(), Box<dyn Error>> {
    let out = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/gallery"), PathBuf::from);
    std::fs::create_dir_all(&out)?;
    let mut index = String::from(HEAD);
    writeln!(
        index,
        "<h1>pltrs gallery</h1>\n<p>Drawn in the {} look.</p>\n<nav>",
        look::appearance().figures
    )?;
    for section in SECTIONS {
        writeln!(index, "<a href=\"#{}\">{}</a>", section.id, section.title)?;
    }
    index.push_str("</nav>\n");
    for section in SECTIONS {
        let mut figures = String::new();
        let mut files = Vec::new();
        for entry in section.entries.iter().copied().flatten() {
            let file = entry.write(&out)?;
            println!("wrote {}", out.join(&file).display());
            writeln!(
                figures,
                "<figure><a href=\"{file}\"><img src=\"{file}\" alt=\"{0}\"></a>\
                 <figcaption>{0}</figcaption></figure>",
                entry.caption
            )?;
            files.push(file);
        }
        let sheet = sheet::write(&out, section.id, section.title, &files)?;
        println!("wrote {}", out.join(&sheet).display());
        writeln!(
            index,
            "<h2 id=\"{}\">{}</h2>\n<p><a href=\"{sheet}\">contact sheet</a></p>\n{figures}",
            section.id, section.title
        )?;
    }
    index.push_str("</body></html>\n");
    std::fs::write(out.join("index.html"), index)?;
    println!("wrote {}", out.join("index.html").display());
    Ok(())
}
