//! A gallery of plt-rs figures, one function per figure, grouped by topic
//! like matplotlib's gallery.
//!
//! ```text
//! cargo run -p plt --example gallery -- OUTDIR
//! ```
//!
//! Every figure is written to `OUTDIR/<name>.png` (`target/gallery` by
//! default), with an `index.html` showing them all.

#![expect(clippy::print_stdout, reason = "the example reports what it wrote")]

mod bars;
mod data;
mod fields;
mod layout;
mod lines;
mod projections;
mod stats;
mod styles;
mod text;

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use plt::figure::Figure;

/// The result of building one figure.
type Built = Result<Figure, Box<dyn Error>>;

/// One figure of the gallery: its file name, its caption and how it is
/// built.
struct Entry {
    name: &'static str,
    caption: &'static str,
    build: fn() -> Built,
}

/// Every figure, topic by topic.
fn entries() -> Vec<(&'static str, &'static [Entry])> {
    vec![
        ("Lines and areas", lines::ENTRIES),
        ("Bars", bars::ENTRIES),
        ("Statistics", stats::ENTRIES),
        ("Images and fields", fields::ENTRIES),
        ("Text and annotations", text::ENTRIES),
        ("Layout", layout::ENTRIES),
        ("Projections", projections::ENTRIES),
        ("Style sheets", styles::ENTRIES),
    ]
}

fn main() -> Result<(), Box<dyn Error>> {
    let out = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("target/gallery"), PathBuf::from);
    std::fs::create_dir_all(&out)?;
    let mut index = String::from(concat!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>plt-rs gallery</title>\n",
        "<style>body{font-family:sans-serif;margin:2em}figure{display:inline-block;",
        "margin:0 1em 1em 0;width:320px;vertical-align:top}img{width:100%}</style>",
        "</head><body><h1>plt-rs gallery</h1>\n"
    ));
    for (topic, figures) in entries() {
        writeln!(index, "<h2>{topic}</h2>")?;
        for entry in figures {
            let file = format!("{}.png", entry.name);
            (entry.build)()?.savefig(out.join(&file))?;
            println!("wrote {}", out.join(&file).display());
            writeln!(
                index,
                "<figure><img src=\"{file}\" alt=\"{0}\"><figcaption>{0}</figcaption></figure>",
                entry.caption
            )?;
        }
    }
    index.push_str("</body></html>\n");
    std::fs::write(out.join("index.html"), index)?;
    println!("wrote {}", out.join("index.html").display());
    Ok(())
}
