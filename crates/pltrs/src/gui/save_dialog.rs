//! The "Save the figure" dialog of figure windows (matplotlib's
//! `NavigationToolbar2.save_figure` file chooser): a directory listing, a
//! file name and the output format.

use std::path::{Path, PathBuf};

use pltrs_platform::{Context, WindowEvent, WindowId};
use pltrs_ui::{
    Dropdown, Flex, Form, Label, List, ListItem, Overflow, TextInput, UiWindow, WidgetId,
};

use super::dialog::{self, Dialog};
use super::{GuiError, icons};
use crate::figure::{Figure, SaveFormat};

/// How many rows of the directory listing show at first.
const LISTED_ROWS: usize = 10;

/// A format offered: matplotlib's name of it and the extensions of its
/// files (`get_supported_filetypes_grouped`).
#[derive(Clone, Copy, Debug)]
struct Offered {
    format: SaveFormat,
    name: &'static str,
    extensions: &'static str,
}

/// The formats offered, in the order matplotlib's Qt dialog lists them:
/// by name. SVG and its compressed form, which matplotlib groups, are
/// apart, as the format chosen gives the file's extension.
const FORMATS: [Offered; 13] = [
    offered(SaveFormat::Avif, "AV1 Image File Format", "*.avif"),
    offered(SaveFormat::Eps, "Encapsulated Postscript", "*.eps"),
    offered(SaveFormat::Gif, "Graphics Interchange Format", "*.gif"),
    offered(
        SaveFormat::Jpeg,
        "Joint Photographic Experts Group",
        "*.jpeg *.jpg",
    ),
    offered(SaveFormat::Pgf, "PGF code for LaTeX", "*.pgf"),
    offered(SaveFormat::Pdf, "Portable Document Format", "*.pdf"),
    offered(SaveFormat::Png, "Portable Network Graphics", "*.png"),
    offered(SaveFormat::Ps, "Postscript", "*.ps"),
    offered(SaveFormat::Raw, "Raw RGBA bitmap", "*.raw *.rgba"),
    offered(SaveFormat::Svg, "Scalable Vector Graphics", "*.svg"),
    offered(SaveFormat::Svgz, "Scalable Vector Graphics", "*.svgz"),
    offered(SaveFormat::Tiff, "Tagged Image File Format", "*.tif *.tiff"),
    offered(SaveFormat::Webp, "WebP Image Format", "*.webp"),
];

/// A format offered under `name`, for files ending in `extensions`.
const fn offered(format: SaveFormat, name: &'static str, extensions: &'static str) -> Offered {
    Offered {
        format,
        name,
        extensions,
    }
}

/// The label of a format in the list, as matplotlib's Qt dialog writes
/// its filters: `"Portable Network Graphics (*.png)"`.
fn label(offered: &Offered) -> String {
    format!("{} ({})", offered.name, offered.extensions)
}

/// What the dialog's widgets report.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum DialogMessage {
    /// A row of the listing was selected.
    Selected(usize),
    /// A row of the listing was double-clicked.
    Opened(usize),
    /// The file name was edited.
    NameChanged(String),
    /// A format was chosen.
    Format(usize),
    /// Save (the button, or Enter outside the listing).
    Save,
    /// Close without saving.
    Cancel,
}

/// How the dialog ended, once it has.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Outcome {
    /// Still open.
    Open,
    /// The file: written there by a dialog that saves, chosen by one that
    /// only chooses.
    Done(PathBuf),
    /// Closed without saving.
    Cancelled,
}

/// What the dialog does with the file chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Purpose {
    /// Saves the figure there, remembering the directory in
    /// `savefig.directory`.
    Save,
    /// Only hands the file over, for the program to save the figure (as
    /// matplotlib's file choosers do for `save_figure`).
    Choose,
}

/// A row of the listing.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    path: PathBuf,
    /// The name shown (`..` for the parent).
    name: String,
    is_dir: bool,
}

/// The parent (when there is one), then the visible subdirectories and
/// files of `directory`, each sorted by name ignoring case.
fn list_directory(directory: &Path) -> Vec<Entry> {
    let mut entries: Vec<Entry> = std::fs::read_dir(directory)
        .map(|read| {
            read.filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let is_dir = entry.path().is_dir();
                    (!name.starts_with('.')).then(|| Entry {
                        path: entry.path(),
                        name,
                        is_dir,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    if let Some(parent) = directory.parent() {
        entries.insert(
            0,
            Entry {
                path: parent.to_path_buf(),
                name: "..".to_owned(),
                is_dir: true,
            },
        );
    }
    entries
}

/// The format a file name's extension names, if offered.
fn format_of(name: &str) -> Option<usize> {
    let extension = Path::new(name).extension()?.to_str()?;
    let format: SaveFormat = extension.parse().ok()?;
    FORMATS.iter().position(|offered| offered.format == format)
}

/// `name` with its extension replaced by that of `format`.
fn with_extension(name: &str, format: SaveFormat) -> String {
    let stem = match format_of(name) {
        Some(_) => Path::new(name).file_stem().map_or_else(
            || name.to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        ),
        None => name.to_owned(),
    };
    format!("{stem}.{}", format.extension())
}

/// The directory the dialog starts in: `savefig.directory` (`~` is the
/// home directory) when it exists, else the working directory.
fn start_directory() -> PathBuf {
    let setting = crate::rc::params().savefig_directory.clone();
    (!setting.is_empty())
        .then(|| pltrs_core::os::expand_user(Path::new(&setting)))
        .filter(|dir| dir.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The widgets the dialog updates.
#[derive(Debug, Clone, Copy)]
struct Widgets {
    location: WidgetId,
    list: WidgetId,
    name: WidgetId,
    formats: WidgetId,
    status: WidgetId,
}

/// A window to choose where and how to save a figure.
#[derive(Debug)]
pub(super) struct SaveDialog {
    figure: Figure,
    window: UiWindow<DialogMessage>,
    widgets: Widgets,
    directory: PathBuf,
    entries: Vec<Entry>,
    name: String,
    format: usize,
    /// The existing file a first Save asked to replace.
    replacing: Option<PathBuf>,
    purpose: Purpose,
}

/// The format `savefig.format` names, else PNG.
fn default_format() -> usize {
    let rc_format = crate::rc::params().savefig_format.clone();
    format_of(&format!("x.{rc_format}")).unwrap_or(0)
}

impl SaveDialog {
    /// Opens the dialog for `figure` shown in a window titled `title`.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn open(
        cx: &mut Context<'_>,
        figure: &Figure,
        title: &str,
    ) -> Result<Self, GuiError> {
        let format = default_format();
        let name = FORMATS[format].format.file_name_for(title);
        Self::build(cx, figure, start_directory(), name, format, Purpose::Save)
    }

    /// Opens the dialog to choose where to save `figure`, in `directory`
    /// when it is one (else where [`SaveDialog::open`] starts), proposing
    /// the file name `name` (whose extension chooses the format, else
    /// `savefig.format` does): it ends with the file chosen, saving
    /// nothing.
    ///
    /// # Errors
    ///
    /// When the window cannot be created.
    pub(super) fn choose(
        cx: &mut Context<'_>,
        figure: &Figure,
        directory: Option<&Path>,
        name: &str,
    ) -> Result<Self, GuiError> {
        let format = format_of(name).unwrap_or_else(default_format);
        let directory = directory
            .filter(|directory| directory.is_dir())
            .map_or_else(start_directory, Path::to_path_buf);
        let name = name.to_owned();
        Self::build(cx, figure, directory, name, format, Purpose::Choose)
    }

    fn build(
        cx: &mut Context<'_>,
        figure: &Figure,
        directory: PathBuf,
        name: String,
        format: usize,
        purpose: Purpose,
    ) -> Result<Self, GuiError> {
        let entries = list_directory(&directory);
        let mut content = Flex::column().with_spacing(dialog::GAP);
        let mut place = Form::new();
        let location = place.row(
            "Folder",
            Label::new(directory.display().to_string()).with_overflow(Overflow::ElideStart),
        );
        content.push(place, 0.0);
        let listing = List::new(items(&entries), DialogMessage::Selected)
            .on_activate(DialogMessage::Opened)
            .with_visible_rows(LISTED_ROWS);
        let list = content.push(listing, 1.0);
        let mut naming = Form::new();
        let name_field = naming.row(
            "File name",
            TextInput::new(name.clone(), DialogMessage::NameChanged),
        );
        let labels = FORMATS.iter().map(label).collect();
        let mut choice = Dropdown::new(labels, DialogMessage::Format);
        choice.set_chosen(format);
        let formats = naming.row("Format", choice);
        content.push(naming, 0.0);
        let (mut chrome, status) = Dialog::with_status();
        chrome.button("Cancel", DialogMessage::Cancel);
        chrome.primary("Save", DialogMessage::Save);
        let options = dialog::window_options("Save the figure");
        let mut window = dialog::open_fitted(cx, figure, &options, chrome.finish(content))?;
        window.ui_mut().set_focus(Some(name_field));
        let mut dialog = Self {
            figure: figure.clone(),
            window,
            widgets: Widgets {
                location,
                list,
                name: name_field,
                formats,
                status,
            },
            directory,
            entries,
            name,
            format,
            replacing: None,
            purpose,
        };
        dialog.select_stem(cx)?;
        Ok(dialog)
    }

    /// The window.
    pub(super) fn id(&self) -> WindowId {
        self.window.id()
    }

    /// Handles an event of the dialog's window.
    ///
    /// # Errors
    ///
    /// When the window cannot be updated.
    pub(super) fn handle(
        &mut self,
        cx: &mut Context<'_>,
        event: &WindowEvent,
    ) -> Result<Outcome, GuiError> {
        if dialog::dismissed(event, self.window.ui()) {
            return Ok(Outcome::Cancelled);
        }
        for message in self.window.handle(cx, event)? {
            let outcome = self.apply(cx, message)?;
            if outcome != Outcome::Open {
                return Ok(outcome);
            }
        }
        Ok(Outcome::Open)
    }

    fn apply(&mut self, cx: &mut Context<'_>, message: DialogMessage) -> Result<Outcome, GuiError> {
        match message {
            DialogMessage::Selected(row) => {
                if let Some(entry) = self.entries.get(row).filter(|entry| !entry.is_dir) {
                    let name = entry.name.clone();
                    self.set_name(cx, name)?;
                }
            }
            DialogMessage::Opened(row) => {
                let Some(entry) = self.entries.get(row).cloned() else {
                    return Ok(Outcome::Open);
                };
                if entry.is_dir {
                    self.enter(cx, entry.path)?;
                } else {
                    self.set_name(cx, entry.name)?;
                    return self.save(cx);
                }
            }
            DialogMessage::NameChanged(name) => {
                self.replacing = None;
                // A problem with the name before goes with the edit.
                self.set_status(cx, String::new())?;
                if let Some(format) = format_of(&name) {
                    self.format = format;
                    self.window.update(
                        cx,
                        self.widgets.formats,
                        |choice: &mut Dropdown<DialogMessage>| {
                            choice.set_chosen(format);
                        },
                    )?;
                }
                self.name = name;
            }
            DialogMessage::Format(format) => {
                self.format = format;
                let name = with_extension(&self.name, FORMATS[format].format);
                self.set_name(cx, name)?;
            }
            DialogMessage::Save => return self.save(cx),
            DialogMessage::Cancel => return Ok(Outcome::Cancelled),
        }
        Ok(Outcome::Open)
    }

    /// Shows `name` in the name field.
    fn set_name(&mut self, cx: &mut Context<'_>, name: String) -> Result<(), GuiError> {
        self.replacing = None;
        if let Some(format) = format_of(&name) {
            self.format = format;
            self.window.update(
                cx,
                self.widgets.formats,
                |choice: &mut Dropdown<DialogMessage>| {
                    choice.set_chosen(format);
                },
            )?;
        }
        self.window.update(
            cx,
            self.widgets.name,
            |input: &mut TextInput<DialogMessage>| {
                input.set_text(name.clone());
            },
        )?;
        self.name = name;
        self.select_stem(cx)
    }

    /// Selects the name without its extension, ready to be typed over.
    fn select_stem(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let stem = match format_of(&self.name) {
            Some(_) => self.name.rfind('.').unwrap_or(self.name.len()),
            None => self.name.len(),
        };
        self.window.update(
            cx,
            self.widgets.name,
            |input: &mut TextInput<DialogMessage>| {
                input.select(0, stem);
            },
        )?;
        Ok(())
    }

    /// Lists `directory`.
    fn enter(&mut self, cx: &mut Context<'_>, directory: PathBuf) -> Result<(), GuiError> {
        self.entries = list_directory(&directory);
        let shown = items(&self.entries);
        self.window
            .update(cx, self.widgets.list, |list: &mut List<DialogMessage>| {
                list.set_items(shown);
            })?;
        let location = directory.display().to_string();
        self.window
            .update(cx, self.widgets.location, |label: &mut Label| {
                label.set_text(location);
            })?;
        self.directory = directory;
        self.replacing = None;
        Ok(())
    }

    fn set_status(&mut self, cx: &mut Context<'_>, text: String) -> Result<(), GuiError> {
        self.window
            .update(cx, self.widgets.status, |label: &mut Label| {
                label.set_text(text);
            })?;
        Ok(())
    }

    /// Writes the figure where the name says, in the chosen format; an
    /// existing file is replaced only on a second Save.
    fn save(&mut self, cx: &mut Context<'_>) -> Result<Outcome, GuiError> {
        let name = self.name.trim();
        if name.is_empty() {
            self.window.update(
                cx,
                self.widgets.name,
                |input: &mut TextInput<DialogMessage>| input.set_invalid(true),
            )?;
            self.set_status(cx, "Type a file name".to_owned())?;
            return Ok(Outcome::Open);
        }
        // A name may be a path, from the home directory too (`~/...`).
        let mut path = self
            .directory
            .join(pltrs_core::os::expand_user(Path::new(name)));
        if path.is_dir() {
            self.enter(cx, path)?;
            return Ok(Outcome::Open);
        }
        if format_of(name).is_none() {
            path.set_extension(FORMATS[self.format].format.extension());
        }
        if path.exists() && self.replacing.as_ref() != Some(&path) {
            let shown = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            // What to do first: a long name gives way at the end.
            self.set_status(cx, format!("Save again to replace the existing {shown}"))?;
            self.replacing = Some(path);
            return Ok(Outcome::Open);
        }
        if self.purpose == Purpose::Choose {
            return Ok(Outcome::Done(path));
        }
        match self.figure.savefig(&path) {
            Ok(()) => {
                if let Some(directory) = path.parent() {
                    remember_directory(directory);
                }
                Ok(Outcome::Done(path))
            }
            Err(err) => {
                self.set_status(cx, format!("Could not save: {err}"))?;
                Ok(Outcome::Open)
            }
        }
    }

    /// Closes the dialog's window.
    ///
    /// # Errors
    ///
    /// When the window system fails to close it.
    pub(super) fn close(self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        cx.close_window(self.id())?;
        Ok(())
    }
}

/// The rows showing `entries`.
fn items(entries: &[Entry]) -> Vec<ListItem> {
    entries
        .iter()
        .map(|entry| {
            let icon = if entry.is_dir {
                icons::folder()
            } else {
                icons::file()
            };
            ListItem::new(entry.name.clone()).with_icon(icon)
        })
        .collect()
}

/// Makes the next dialog start in `directory`, as matplotlib updates
/// `savefig.directory` unless it is empty.
fn remember_directory(directory: &Path) {
    let keep = !crate::rc::params().savefig_directory.is_empty();
    if keep {
        let directory = directory.display().to_string();
        crate::rc::update(|rc| rc.savefig_directory = directory);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_formats() {
        let format = |name| format_of(name).map(|index| FORMATS[index].format);
        assert_eq!(format("plot.SVG"), Some(SaveFormat::Svg));
        assert_eq!(format("plot.svgz"), Some(SaveFormat::Svgz));
        assert_eq!(format("frame.webp"), Some(SaveFormat::Webp));
        assert_eq!(format("frame.avif"), Some(SaveFormat::Avif));
        assert_eq!(format("photo.jpeg"), Some(SaveFormat::Jpeg));
        assert_eq!(format("scan.tiff"), Some(SaveFormat::Tiff));
        assert_eq!(format("pixels.rgba"), Some(SaveFormat::Raw));
        assert_eq!(format_of("notes.txt"), None);
        assert_eq!(
            label(&FORMATS[3]),
            "Joint Photographic Experts Group (*.jpeg *.jpg)"
        );
        let names: Vec<&str> = FORMATS.iter().map(|offered| offered.name).collect();
        assert!(names.is_sorted(), "by name, as Qt's dialog sorts them");
        assert_eq!(with_extension("plot.png", SaveFormat::Pdf), "plot.pdf");
        assert_eq!(with_extension("plot.v2", SaveFormat::Eps), "plot.v2.eps");
    }

    #[test]
    fn listings_put_directories_first() {
        let dir = std::env::temp_dir().join(format!("pltrs-save-dialog-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Sub")).unwrap();
        for name in ["b.png", "A.svg", ".hidden"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        let names: Vec<String> = list_directory(&dir).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["..", "Sub", "A.svg", "b.png"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
