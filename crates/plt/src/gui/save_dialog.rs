//! The "Save the figure" dialog of figure windows (matplotlib's
//! `NavigationToolbar2.save_figure` file chooser): a directory listing, a
//! file name and the output format.

use std::path::{Path, PathBuf};

use plt_platform::{ButtonState, Context, Key, LogicalSize, WindowEvent, WindowId, WindowOptions};
use plt_ui::{Button, Choice, Flex, Label, List, ListItem, TextInput, UiWindow, WidgetId};

use super::{GuiError, icons};
use crate::figure::{Figure, SaveFormat};

/// The formats offered, in the order shown.
const FORMATS: [SaveFormat; 5] = [
    SaveFormat::Png,
    SaveFormat::Svg,
    SaveFormat::Pdf,
    SaveFormat::Ps,
    SaveFormat::Eps,
];

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
    /// Save (the button, or Enter in the name).
    Save,
    /// Close without saving.
    Cancel,
}

/// How the dialog ended, once it has.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Outcome {
    /// Still open.
    Open,
    /// The figure was written here.
    Saved(PathBuf),
    /// Closed without saving.
    Cancelled,
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
    FORMATS.iter().position(|&f| f == format)
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

/// matplotlib's `get_default_filename`: the window title with unsafe
/// characters replaced, and the default format's extension.
fn default_name(title: &str, format: SaveFormat) -> String {
    let base: String = title
        .chars()
        .map(|c| if "<>:\"/\\|?*\0 ".contains(c) { '_' } else { c })
        .collect();
    let base = if base.is_empty() {
        "image".to_owned()
    } else {
        base
    };
    format!("{base}.{}", format.extension())
}

/// The directory the dialog starts in: `savefig.directory` (`~` is the
/// home directory) when it exists, else the working directory.
fn start_directory() -> PathBuf {
    let setting = crate::rc::params().savefig_directory.clone();
    let expanded = match setting.strip_prefix('~') {
        Some(rest) => std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(rest.trim_start_matches('/'))),
        None => (!setting.is_empty()).then(|| PathBuf::from(&setting)),
    };
    expanded
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
        let rc_format = crate::rc::params().savefig_format.clone();
        let format = format_of(&format!("x.{rc_format}")).unwrap_or(0);
        let name = default_name(title, FORMATS[format]);
        let directory = start_directory();
        let entries = list_directory(&directory);
        let mut root = Flex::column().with_padding(10.0).with_spacing(8.0);
        let location = root.push(Label::new(directory.display().to_string()), 0.0);
        let list = root.push(
            List::new(items(&entries), DialogMessage::Selected).on_activate(DialogMessage::Opened),
            1.0,
        );
        let mut name_row = Flex::row().with_spacing(8.0);
        name_row.push(Label::new("File name"), 0.0);
        let name_field = name_row.push(
            TextInput::new(name.clone(), DialogMessage::NameChanged)
                .on_submit(|_| DialogMessage::Save),
            1.0,
        );
        root.push(name_row, 0.0);
        let mut buttons = Flex::row().with_spacing(8.0);
        let labels = FORMATS
            .iter()
            .map(|f| f.extension().to_uppercase())
            .collect();
        let mut choice = Choice::new(labels, DialogMessage::Format);
        choice.set_chosen(format);
        let formats = buttons.push(choice, 0.0);
        let status = buttons.push(Label::new(""), 1.0);
        buttons.push(Button::new("Cancel", DialogMessage::Cancel), 0.0);
        buttons.push(Button::new("Save", DialogMessage::Save).primary(), 0.0);
        root.push(buttons, 0.0);
        let options = WindowOptions::new("Save the figure")
            .with_app_id("plt-rs")
            .with_icon(icons::window_icon()?)
            .with_inner_size(LogicalSize::new(600.0, 440.0))
            .with_min_inner_size(LogicalSize::new(420.0, 300.0));
        let mut window = UiWindow::open(cx, &options, root)?;
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
        match event {
            WindowEvent::CloseRequested => return Ok(Outcome::Cancelled),
            WindowEvent::Key(key)
                if key.key == Key::Escape && key.state == ButtonState::Pressed =>
            {
                return Ok(Outcome::Cancelled);
            }
            _ => {}
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
                if let Some(format) = format_of(&name) {
                    self.format = format;
                    self.window.update(
                        cx,
                        self.widgets.formats,
                        |choice: &mut Choice<DialogMessage>| {
                            choice.set_chosen(format);
                        },
                    )?;
                }
                self.name = name;
            }
            DialogMessage::Format(format) => {
                self.format = format;
                let name = with_extension(&self.name, FORMATS[format]);
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
                |choice: &mut Choice<DialogMessage>| {
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
            self.set_status(cx, "Type a file name".to_owned())?;
            return Ok(Outcome::Open);
        }
        let mut path = self.directory.join(name);
        if path.is_dir() {
            self.enter(cx, path)?;
            return Ok(Outcome::Open);
        }
        if format_of(name).is_none() {
            path.set_extension(FORMATS[self.format].extension());
        }
        if path.exists() && self.replacing.as_ref() != Some(&path) {
            let shown = path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            self.set_status(cx, format!("{shown} exists: Save again to replace it"))?;
            self.replacing = Some(path);
            return Ok(Outcome::Open);
        }
        match self.figure.savefig(&path) {
            Ok(()) => {
                remember_directory(&self.directory);
                Ok(Outcome::Saved(path))
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
        assert_eq!(default_name("Figure 1", SaveFormat::Png), "Figure_1.png");
        assert_eq!(default_name("", SaveFormat::Pdf), "image.pdf");
        assert_eq!(format_of("plot.SVG"), Some(1));
        assert_eq!(format_of("notes.txt"), None);
        assert_eq!(with_extension("plot.png", SaveFormat::Pdf), "plot.pdf");
        assert_eq!(with_extension("plot.v2", SaveFormat::Eps), "plot.v2.eps");
    }

    #[test]
    fn listings_put_directories_first() {
        let dir = std::env::temp_dir().join(format!("plt-save-dialog-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Sub")).unwrap();
        for name in ["b.png", "A.svg", ".hidden"] {
            std::fs::write(dir.join(name), b"").unwrap();
        }
        let names: Vec<String> = list_directory(&dir).into_iter().map(|e| e.name).collect();
        assert_eq!(names, ["..", "Sub", "A.svg", "b.png"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
