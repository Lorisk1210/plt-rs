//! One axes in the figure options dialog: its titles, limits, labels and
//! scales, its curves and its images (matplotlib's `figure_edit` pages).

use pltrs_ui::{Checkbox, Flex, Spacing, Stack, Tabs, UiWindow, WidgetId};

use super::OptionsMessage;
use super::artists::{
    CurveFields, CurvePlan, Detail, ImageFields, ImagePlan, apply_curve, apply_image, curves_panel,
    edited_lines, images_panel, read_curve, read_image,
};
use super::fields::{FieldError, text_of};
use super::general::{AxisFields, AxisPlan, general_form, read_axis};
use crate::axes::{Axes, TitleLocation};
use crate::legend::LegendOptions;

/// Everything a page sets, read and validated before anything changes.
#[derive(Debug)]
pub(super) struct PagePlan {
    title: String,
    axes: Vec<AxisPlan>,
    legend: bool,
    curves: Vec<CurvePlan>,
    images: Vec<ImagePlan>,
}

/// The widgets of one axes.
#[derive(Debug)]
pub(super) struct AxesPage {
    axes: Axes,
    /// The tab bar, when the axes have curves or images.
    tabs: Option<WidgetId>,
    panels: WidgetId,
    title: WidgetId,
    /// Where the edited title sits.
    title_location: TitleLocation,
    axis_fields: [AxisFields; 2],
    legend: WidgetId,
    curves: Vec<CurveFields>,
    curve_detail: Option<Detail>,
    images: Vec<ImageFields>,
    image_detail: Option<Detail>,
}

impl AxesPage {
    /// The page for `axes`, number `page` of the dialog, and its widgets.
    pub(super) fn build(axes: &Axes, page: usize) -> (Self, Flex<OptionsMessage>) {
        let mut root = Flex::column().with_spacing(Spacing::Lg);
        let lines = edited_lines(axes);
        let images = axes.images();
        let mut tabs = vec!["Axes".to_owned()];
        if !lines.is_empty() {
            tabs.push("Curves".to_owned());
        }
        if !images.is_empty() {
            tabs.push("Images, etc.".to_owned());
        }
        let tabs = (tabs.len() > 1).then(|| {
            root.push(
                Tabs::new(tabs, move |tab| OptionsMessage::Tab { page, tab }),
                0.0,
            )
        });
        let mut panels = Stack::new();
        let (title_location, shown) = axes.shown_title();
        let (title, axis_fields, legend, general) = general_form(axes, &shown);
        panels.push(general);
        let (curves, curve_detail) = if lines.is_empty() {
            (Vec::new(), None)
        } else {
            let (curves, panel, detail) = curves_panel(&lines, page);
            panels.push(panel);
            (curves, Some(detail))
        };
        let (image_fields, image_detail) = if images.is_empty() {
            (Vec::new(), None)
        } else {
            let (fields, panel, detail) = images_panel(&images, page);
            panels.push(panel);
            (fields, Some(detail))
        };
        let panels = root.push(panels, 1.0);
        let page = Self {
            axes: axes.clone(),
            tabs,
            panels,
            title,
            title_location,
            axis_fields,
            legend,
            curves,
            curve_detail,
            images: image_fields,
            image_detail,
        };
        (page, root)
    }

    /// The field of the title, the page's first field.
    pub(super) fn title_field(&self) -> WidgetId {
        self.title
    }

    /// Shows tab `tab`.
    pub(super) fn show_tab(&self, window: &mut UiWindow<OptionsMessage>, tab: usize) {
        show(window, self.panels, tab);
    }

    /// Shows the fields of curve `index`.
    pub(super) fn show_curve(&self, window: &mut UiWindow<OptionsMessage>, index: usize) {
        if let Some(detail) = self.curve_detail {
            detail.show(window, index);
        }
    }

    /// Shows the fields of image `index`.
    pub(super) fn show_image(&self, window: &mut UiWindow<OptionsMessage>, index: usize) {
        if let Some(detail) = self.image_detail {
            detail.show(window, index);
        }
    }

    /// Shows text field `field` if it is one of the page's, as choosing
    /// its tab and its curve or image would; returns whether it is.
    pub(super) fn reveal(&self, window: &mut UiWindow<OptionsMessage>, field: WidgetId) -> bool {
        let mut general = std::iter::once(self.title)
            .chain(self.axis_fields.iter().flat_map(AxisFields::text_fields));
        let curve = self
            .curves
            .iter()
            .position(|curve| curve.text_fields().contains(&field));
        let image = self
            .images
            .iter()
            .position(|image| image.text_fields().contains(&field));
        let images_tab = 1 + usize::from(self.curve_detail.is_some());
        let (tab, artist) = if general.any(|id| id == field) {
            (0, None)
        } else if let (Some(index), Some(detail)) = (curve, self.curve_detail) {
            (1, Some((detail, index)))
        } else if let (Some(index), Some(detail)) = (image, self.image_detail) {
            (images_tab, Some((detail, index)))
        } else {
            return false;
        };
        if let Some(tabs) = self.tabs
            && let Some(tabs) = window.ui_mut().widget_mut::<Tabs<OptionsMessage>>(tabs)
        {
            tabs.set_selected(tab);
        }
        self.show_tab(window, tab);
        if let Some((detail, index)) = artist {
            detail.show(window, index);
        }
        true
    }

    /// Reads and checks every field.
    pub(super) fn read(
        &self,
        window: &mut UiWindow<OptionsMessage>,
    ) -> Result<PagePlan, FieldError> {
        let axes = self
            .axis_fields
            .iter()
            .map(|fields| read_axis(window, fields))
            .collect::<Result<_, _>>()?;
        let legend = window
            .ui_mut()
            .widget_mut::<Checkbox<OptionsMessage>>(self.legend)
            .is_some_and(|checkbox| checkbox.checked());
        Ok(PagePlan {
            title: text_of(window, self.title),
            axes,
            legend,
            curves: self
                .curves
                .iter()
                .map(|fields| read_curve(window, fields))
                .collect::<Result<_, _>>()?,
            images: self
                .images
                .iter()
                .map(|fields| read_image(window, fields))
                .collect::<Result<_, _>>()?,
        })
    }

    /// Applies `plan`; returns whether the view limits changed.
    pub(super) fn apply(&self, plan: PagePlan) -> bool {
        let ax = &self.axes;
        let before = (ax.xlim(), ax.ylim());
        ax.set_title_at(self.title_location, plan.title);
        for axis in plan.axes {
            axis.apply(ax);
        }
        for (fields, plan) in self.curves.iter().zip(plan.curves) {
            apply_curve(&fields.line, plan);
        }
        for (fields, plan) in self.images.iter().zip(plan.images) {
            apply_image(&fields.image, plan);
        }
        if plan.legend {
            // A figure with nothing to list keeps no legend, as in
            // matplotlib.
            let _ = ax.legend(LegendOptions::default());
        }
        before != (ax.xlim(), ax.ylim())
    }
}

/// Shows page `index` of stack `stack`.
fn show(window: &mut UiWindow<OptionsMessage>, stack: WidgetId, index: usize) {
    if let Some(stack) = window.ui_mut().widget_mut::<Stack<OptionsMessage>>(stack) {
        stack.show(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::lines::LineProps;

    #[test]
    fn fields_are_checked_then_applied() {
        use pltrs_platform::{EventLoop, WindowOptions};
        use pltrs_ui::TextInput;

        let ax = Figure::new().subplot();
        let line = ax
            .plot_with(
                vec![0.0, 1.0],
                vec![0.0, 1.0],
                LineProps::default().label("a"),
            )
            .unwrap();
        let (page, root) = AxesPage::build(&ax, 0);
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut window = UiWindow::open(
            &mut event_loop.context(),
            &WindowOptions::new("options"),
            root,
        )
        .unwrap();
        let type_into = |window: &mut UiWindow<OptionsMessage>, id: WidgetId, text: &str| {
            window
                .ui_mut()
                .widget_mut::<TextInput<OptionsMessage>>(id)
                .unwrap()
                .set_text(text);
        };
        let x_max = page.axis_fields[0].text_fields()[1];
        type_into(&mut window, page.title, "Edited");
        type_into(&mut window, x_max, "ten");
        let problem = page.read(&mut window).unwrap_err();
        assert_eq!(problem.message, "X max: \"ten\" is not a number");
        assert_eq!(problem.field, x_max, "the problem names its field");
        type_into(&mut window, x_max, "inf");
        let problem = page.read(&mut window).unwrap_err();
        assert_eq!(problem.message, "X max: \"inf\" is not a finite number");
        type_into(&mut window, x_max, "10");
        type_into(&mut window, page.curves[0].width, "3");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan), "the x limits changed");
        assert_eq!(ax.title(TitleLocation::Center), "Edited");
        assert_eq!(ax.xlim().1, 10.0);
        assert_eq!(line.linewidth(), 3.0);
    }

    #[test]
    fn limits_show_rounded_and_stay_exact_unless_edited() {
        use pltrs_platform::{EventLoop, WindowOptions};
        use pltrs_ui::TextInput;

        let ax = Figure::with_size(4.0, 3.0, 100.0).subplot();
        ax.plot([0.0, 9.8], [0.0, 1.0]).unwrap();
        let (x0, x1) = ax.xlim();
        assert_eq!(
            (x0, x1),
            (-0.490_000_000_000_000_05, 10.290_000_000_000_001)
        );
        let ylim = ax.ylim();
        let (page, root) = AxesPage::build(&ax, 0);
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut window = UiWindow::open(
            &mut event_loop.context(),
            &WindowOptions::new("options"),
            root,
        )
        .unwrap();
        let [x, y] = page.axis_fields.clone();
        assert_eq!(text_of(&mut window, x.text_fields()[0]), "-0.49");
        assert_eq!(text_of(&mut window, x.text_fields()[1]), "10.29");
        assert_eq!(text_of(&mut window, y.text_fields()[1]), "1.05");
        let plan = page.read(&mut window).unwrap();
        assert!(!page.apply(plan), "no limit changed");
        assert_eq!((ax.xlim(), ax.ylim()), ((x0, x1), ylim));
        window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(x.text_fields()[1])
            .unwrap()
            .set_text("12");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan));
        assert_eq!(ax.xlim(), (x0, 12.0), "the unedited minimum stays exact");
    }

    #[test]
    fn the_title_is_edited_where_it_is_shown() {
        use pltrs_platform::{EventLoop, WindowOptions};
        use pltrs_ui::TextInput;

        // The modern look puts titles on the left.
        let ax = Figure::new().subplot();
        ax.set_title_at(TitleLocation::Left, "Left");
        let (page, root) = AxesPage::build(&ax, 0);
        assert_eq!(page.title_location, TitleLocation::Left);
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut window = UiWindow::open(
            &mut event_loop.context(),
            &WindowOptions::new("options"),
            root,
        )
        .unwrap();
        assert_eq!(text_of(&mut window, page.title), "Left");
        window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(page.title)
            .unwrap()
            .set_text("Moved");
        let plan = page.read(&mut window).unwrap();
        page.apply(plan);
        assert_eq!(ax.title(TitleLocation::Left), "Moved");
        assert_eq!(
            ax.title(TitleLocation::Center),
            "",
            "nothing appears elsewhere"
        );
    }

    #[test]
    fn limits_of_dates_show_and_read_as_dates() {
        use pltrs_platform::{EventLoop, WindowOptions};
        use pltrs_ui::TextInput;

        use crate::dates::DateTime;

        let ax = Figure::new().subplot();
        let start = DateTime::new(2020, 1, 1, 0, 0, 0, 0).unwrap();
        let end = DateTime::new(2020, 1, 3, 12, 0, 0, 0).unwrap();
        ax.plot(&[start, end][..], &[0.0, 1.0][..]).unwrap();
        ax.set_xlim(start, end).unwrap();
        let (page, root) = AxesPage::build(&ax, 0);
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut window = UiWindow::open(
            &mut event_loop.context(),
            &WindowOptions::new("options"),
            root,
        )
        .unwrap();
        let [x, y] = page.axis_fields.clone();
        assert_eq!(
            text_of(&mut window, x.text_fields()[0]),
            "2020-01-01 00:00:00"
        );
        assert_eq!(
            text_of(&mut window, x.text_fields()[1]),
            "2020-01-03 12:00:00"
        );
        assert!(
            !text_of(&mut window, y.text_fields()[1]).contains('-'),
            "numbers"
        );
        window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(x.text_fields()[1])
            .unwrap()
            .set_text("2020-01-05");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan));
        assert_eq!(ax.xlim(), (18_262.0, 18_266.0));
    }

    #[test]
    fn limits_of_dates_show_and_read_in_the_timezone_rc_param() {
        use pltrs_platform::{EventLoop, WindowOptions};
        use pltrs_ui::TextInput;

        use crate::dates::{DateTime, TimeZone};

        if TimeZone::named("Asia/Tokyo").is_err() {
            return;
        }
        crate::rc::context(|| {
            crate::rc::set("timezone", "Asia/Tokyo").unwrap();
            let ax = Figure::new().subplot();
            let start = DateTime::new(2020, 1, 1, 0, 0, 0, 0).unwrap();
            let end = DateTime::new(2020, 1, 3, 12, 0, 0, 0).unwrap();
            ax.plot(&[start, end][..], &[0.0, 1.0][..]).unwrap();
            ax.set_xlim(start, end).unwrap();
            let (page, root) = AxesPage::build(&ax, 0);
            let (mut event_loop, _handle) = EventLoop::headless();
            let mut window = UiWindow::open(
                &mut event_loop.context(),
                &WindowOptions::new("options"),
                root,
            )
            .unwrap();
            let x = page.axis_fields[0].clone();
            // Tokyo is nine hours ahead of UTC.
            assert_eq!(
                text_of(&mut window, x.text_fields()[0]),
                "2020-01-01 09:00:00"
            );
            assert_eq!(
                text_of(&mut window, x.text_fields()[1]),
                "2020-01-03 21:00:00"
            );
            window
                .ui_mut()
                .widget_mut::<TextInput<OptionsMessage>>(x.text_fields()[1])
                .unwrap()
                .set_text("2020-01-05 09:00");
            let plan = page.read(&mut window).unwrap();
            assert!(page.apply(plan));
            assert_eq!(ax.xlim(), (18_262.0, 18_266.0), "typed in Tokyo time");
        });
    }

    #[test]
    fn a_refused_field_is_shown_on_its_axes_tab_and_curve() {
        use pltrs_platform::EventLoop;
        use pltrs_ui::{Dropdown, TextInput};

        use super::super::{OptionsDialog, Outcome};

        let figure = Figure::with_size(6.0, 3.0, 100.0);
        let axs = figure.subplots(1, 2).unwrap();
        for label in ["first", "second"] {
            axs[(0, 0)]
                .plot_with(
                    vec![0.0, 1.0],
                    vec![0.0, 1.0],
                    LineProps::default().label(label),
                )
                .unwrap();
        }
        axs[(0, 1)].plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        let (mut event_loop, _handle) = EventLoop::headless();
        let mut cx = event_loop.context();
        let mut dialog = OptionsDialog::open(&mut cx, &figure).unwrap().unwrap();
        let width = dialog.pages[0].curves[1].width;
        dialog
            .window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(width)
            .unwrap()
            .set_text("thick");
        // The second axes are shown when the first ones' second curve is
        // refused.
        dialog.show_axes(1);
        assert_eq!(dialog.apply(&mut cx, true).unwrap(), Outcome::Open);
        let ui = dialog.window.ui_mut();
        let axes_choice = dialog.axes_choice.unwrap();
        let chosen = ui.widget_mut::<Dropdown<OptionsMessage>>(axes_choice);
        assert_eq!(chosen.map(|choice| choice.chosen()), Some(0));
        let tabs = dialog.pages[0].tabs.unwrap();
        let tab = ui.widget_mut::<Tabs<OptionsMessage>>(tabs);
        assert_eq!(tab.map(|tabs| tabs.selected()), Some(1), "the Curves tab");
        assert_eq!(ui.focus(), Some(width));
        let field = ui.widget_mut::<TextInput<OptionsMessage>>(width).unwrap();
        assert!(field.invalid());
        assert_eq!(field.selection(), Some((0, 5)), "ready to be typed over");
        ui.paint().unwrap();
        let shown = ui.widget_rect(width).unwrap();
        assert!(shown.width > 0.0, "the field is laid out: {shown:?}");
    }
}
