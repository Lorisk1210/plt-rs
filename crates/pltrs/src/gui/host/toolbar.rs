//! The toolbar of a hosted window: the program's buttons in groups
//! separated by lines, then a status line (matplotlib's
//! `NavigationToolbar2` and `ToolContainerBase` widgets). It is kept as
//! the program describes it until its window is shown, then updated in
//! place.

use pltrs_platform::Context;
use pltrs_raster::Pixmap;
use pltrs_ui::{
    Flex, Icon, Label, Picture, Separator, Spacing, ToolButton, ToolIcon, UiWindow, WidgetId,
};

use super::window::Message;
use crate::gui::window::{button_row, push_status};
use crate::gui::{GuiError, icons, tool_icons};
use crate::image::RgbaImage;
use crate::look::ChromeLook;

/// The side of a toolbar picture in logical pixels, as matplotlib's Tk
/// toolbar sizes its images.
const PICTURE_SIDE: f64 = 24.0;

/// The picture of a toolbar button.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolImage {
    /// One of matplotlib's toolbar images (`home`, `back`, `forward`,
    /// `move`, `zoom_to_rect`, `subplots`, `qt4_editor_options`,
    /// `filesave`, `help`; see [`ToolImage::is_drawn`]), drawn in the
    /// chrome of the current [look](crate::look): pltrs' line icons in the
    /// modern chrome, matplotlib's images in matplotlib's.
    Named(String),
    /// A square image of the program's, in straight RGBA, drawn 24
    /// logical pixels wide.
    Bitmap(RgbaImage),
}

impl ToolImage {
    /// Whether pltrs draws matplotlib's toolbar image `name` itself.
    pub fn is_drawn(name: &str) -> bool {
        tool_icons::named(name, ChromeLook::Modern).is_some()
    }

    /// The icon in the chrome `look`; a name pltrs does not draw shows an
    /// empty button.
    fn icon(&self, look: ChromeLook) -> ToolIcon {
        match self {
            Self::Named(name) => tool_icons::named(name, look)
                .unwrap_or_else(|| ToolIcon::Vector(Icon::new(icons::GRID))),
            Self::Bitmap(image) => {
                let bitmap = Pixmap::from_vec(
                    image.width() as u32,
                    image.height() as u32,
                    image.data().to_vec(),
                )
                .expect("an image fills a pixmap of its size");
                Picture::new(PICTURE_SIDE, vec![bitmap]).into()
            }
        }
    }
}

/// A button of a hosted window's toolbar, as the program describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolItem {
    /// What a click reports ([`HostEvent::ToolClicked`](super::HostEvent));
    /// several buttons may share a name.
    pub name: String,
    /// The group the button joins: groups show in the order they were
    /// first used, separated by lines.
    pub group: String,
    /// Shown when the pointer rests on the button.
    pub tooltip: String,
    /// The button's picture.
    pub image: ToolImage,
}

/// A button, with its state and its widget once shown.
#[derive(Debug)]
struct Button {
    name: String,
    tooltip: String,
    icon: ToolIcon,
    checked: bool,
    enabled: bool,
    widget: Option<WidgetId>,
}

impl Button {
    /// The widget showing the button.
    fn widget(&self) -> ToolButton<Message> {
        let mut button = ToolButton::new(self.icon.clone(), Message::Tool(self.name.clone()))
            .with_tooltip(self.tooltip.clone());
        button.set_checked(self.checked);
        button.set_enabled(self.enabled);
        button
    }
}

/// A group of buttons, with its row once shown.
#[derive(Debug)]
struct Group {
    name: String,
    buttons: Vec<Button>,
    widget: Option<WidgetId>,
}

/// The widgets of a shown toolbar.
#[derive(Clone, Copy, Debug)]
struct Shown {
    /// The row holding the groups, the separators between them and the
    /// status line.
    row: WidgetId,
    status: WidgetId,
}

/// The toolbar of a hosted window. Its methods take the window once it
/// is shown, to update its widgets too.
#[derive(Debug)]
pub(super) struct Toolbar {
    look: ChromeLook,
    groups: Vec<Group>,
    message: String,
    shown: Option<Shown>,
}

impl Toolbar {
    /// An empty toolbar drawing its icons in the chrome `look`.
    pub(super) fn new(look: ChromeLook) -> Self {
        Self {
            look,
            groups: Vec::new(),
            message: String::new(),
            shown: None,
        }
    }

    /// Appends a separator and the toolbar to `root`, as its window opens;
    /// [`Toolbar::show_message`] then fills the status line.
    pub(super) fn build(&mut self, root: &mut Flex<Message>) {
        let mut row = button_row();
        for (index, group) in self.groups.iter_mut().enumerate() {
            if index > 0 {
                row.push(Separator::vertical(), 0.0);
            }
            let mut buttons = group_row();
            for button in &mut group.buttons {
                button.widget = Some(buttons.push(button.widget(), 0.0));
            }
            group.widget = Some(row.push(buttons, 0.0));
        }
        let status = push_status(&mut row);
        root.push(Separator::horizontal(), 0.0);
        let row = root.push(row, 0.0);
        self.shown = Some(Shown { row, status });
    }

    /// Shows the message in the status line of the toolbar of `window`.
    pub(super) fn show_message(
        &self,
        window: &mut UiWindow<Message>,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        if let Some(shown) = self.shown {
            let text = self.message.clone();
            window.update(cx, shown.status, |label: &mut Label| label.set_text(text))?;
        }
        Ok(())
    }

    /// Adds the button `item` to its group at `position` among the
    /// group's buttons (at the end with `None`), unchecked and enabled.
    pub(super) fn add(
        &mut self,
        item: ToolItem,
        position: Option<usize>,
        mut window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        let index = match self.groups.iter().position(|g| g.name == item.group) {
            Some(index) => index,
            None => self.add_group(item.group, window.as_deref_mut(), cx)?,
        };
        let mut button = Button {
            icon: item.image.icon(self.look),
            name: item.name,
            tooltip: item.tooltip,
            checked: false,
            enabled: true,
            widget: None,
        };
        let group = &mut self.groups[index];
        let at = position.map_or(group.buttons.len(), |p| p.min(group.buttons.len()));
        if let (Some(window), Some(row)) = (window, group.widget) {
            let widget = button.widget();
            button.widget = window.update(cx, row, |buttons: &mut Flex<Message>| {
                buttons.insert(at, widget, 0.0)
            })?;
        }
        group.buttons.insert(at, button);
        Ok(())
    }

    /// Starts the group `name` after the others; returns its index.
    fn add_group(
        &mut self,
        name: String,
        window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<usize, GuiError> {
        let index = self.groups.len();
        let mut widget = None;
        if let (Some(window), Some(shown)) = (window, self.shown) {
            widget = window.update(cx, shown.row, |row: &mut Flex<Message>| {
                // The row holds the groups, a separator between each two,
                // then the status line.
                let mut at = row.len().saturating_sub(1);
                if index > 0 {
                    row.insert(at, Separator::vertical(), 0.0);
                    at += 1;
                }
                row.insert(at, group_row(), 0.0)
            })?;
        }
        self.groups.push(Group {
            name,
            buttons: Vec::new(),
            widget,
        });
        Ok(index)
    }

    /// Removes every button called `name`; returns whether there was one.
    pub(super) fn remove(
        &mut self,
        name: &str,
        mut window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<bool, GuiError> {
        let mut removed = false;
        for group in &mut self.groups {
            for button in group.buttons.iter().filter(|button| button.name == name) {
                removed = true;
                if let (Some(window), Some(row), Some(widget)) =
                    (window.as_deref_mut(), group.widget, button.widget)
                {
                    window.update(cx, row, |buttons: &mut Flex<Message>| {
                        buttons.remove(widget)
                    })?;
                }
            }
            group.buttons.retain(|button| button.name != name);
        }
        Ok(removed)
    }

    /// Checks or unchecks the buttons called `name` (the state of toggle
    /// tools: matplotlib's `toggle_toolitem`, or pan and zoom).
    pub(super) fn set_checked(
        &mut self,
        name: &str,
        checked: bool,
        window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        self.update_buttons(name, window, cx, |button| button.checked = checked)
    }

    /// Enables or disables the buttons called `name`.
    pub(super) fn set_enabled(
        &mut self,
        name: &str,
        enabled: bool,
        window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        self.update_buttons(name, window, cx, |button| button.enabled = enabled)
    }

    /// Changes the buttons called `name` with `change`, and their widgets.
    fn update_buttons(
        &mut self,
        name: &str,
        mut window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
        change: impl Fn(&mut Button),
    ) -> Result<(), GuiError> {
        let buttons = self.groups.iter_mut().flat_map(|group| &mut group.buttons);
        for button in buttons.filter(|button| button.name == name) {
            change(button);
            if let (Some(window), Some(widget)) = (window.as_deref_mut(), button.widget) {
                let (checked, enabled) = (button.checked, button.enabled);
                window.update(cx, widget, |shown: &mut ToolButton<Message>| {
                    shown.set_checked(checked);
                    shown.set_enabled(enabled);
                })?;
            }
        }
        Ok(())
    }

    /// Shows `text` in the status line.
    pub(super) fn set_message(
        &mut self,
        text: &str,
        window: Option<&mut UiWindow<Message>>,
        cx: &mut Context<'_>,
    ) -> Result<(), GuiError> {
        text.clone_into(&mut self.message);
        match window {
            Some(window) => self.show_message(window, cx),
            None => Ok(()),
        }
    }

    /// The message of the status line.
    pub(super) fn message(&self) -> &str {
        &self.message
    }

    /// The groups and the names of their buttons, in order.
    pub(super) fn layout(&self) -> Vec<(String, Vec<String>)> {
        self.groups
            .iter()
            .map(|group| {
                let names = group.buttons.iter().map(|b| b.name.clone()).collect();
                (group.name.clone(), names)
            })
            .collect()
    }

    /// Whether the first button called `name` is checked, if there is one.
    pub(super) fn checked(&self, name: &str) -> Option<bool> {
        self.groups
            .iter()
            .flat_map(|group| &group.buttons)
            .find(|button| button.name == name)
            .map(|button| button.checked)
    }
}

/// An empty row for the buttons of a group, spaced as the toolbar's.
fn group_row() -> Flex<Message> {
    button_row().with_padding(Spacing::None)
}
