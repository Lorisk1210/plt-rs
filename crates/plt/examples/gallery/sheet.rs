//! Contact sheets: the figures of a section as thumbnails on one figure,
//! read back with `imread` and shown with `imshow`.

use std::error::Error;
use std::path::Path;

use plt::axes::ImshowOptions;
use plt::figure::Figure;
use plt::image::imread;

/// Thumbnails per row.
const COLUMNS: usize = 4;

/// Writes the images `files` of `dir` as thumbnails titled with their
/// names on one figure headed `title`, to `dir/sheet-<id>.png`; returns
/// the file name.
pub(crate) fn write(
    dir: &Path,
    id: &str,
    title: &str,
    files: &[String],
) -> Result<String, Box<dyn Error>> {
    let rows = files.len().div_ceil(COLUMNS);
    let fig = Figure::with_size(16.0, 3.2 * rows as f64 + 0.6, 60.0);
    let axs = fig.subplots(rows, COLUMNS)?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        ax.set_axis_off();
        if let Some(file) = files.get(i) {
            ax.imshow(imread(dir.join(file))?, ImshowOptions::default())?;
            ax.set_title(file.trim_end_matches(".png"));
        }
    }
    fig.suptitle(title);
    let sheet = format!("sheet-{id}.png");
    fig.savefig(dir.join(&sheet))?;
    Ok(sheet)
}
