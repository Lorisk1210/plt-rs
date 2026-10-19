//! One axes in the figure options dialog: its titles, limits, labels and
//! scales, its curves and its images (matplotlib's `figure_edit` pages).

use plt_ui::{Checkbox, Flex, Spacing, Stack, Tabs, UiWindow, WidgetId};

use super::OptionsMessage;
use super::artists::{
    CurveFields, CurvePlan, ImageFields, ImagePlan, apply_curve, apply_image, curves_panel,
    edited_lines, images_panel, read_curve, read_image,
};
use super::fields::{FieldError, Form, chosen, number_text, parse_number, text_of};
use crate::axes::{Axes, AxisName, TitleLocation};
use crate::legend::LegendOptions;

/// The scales offered, as matplotlib names them.
const SCALES: [&str; 4] = ["linear", "log", "symlog", "logit"];

/// The fields of one axis.
#[derive(Debug, Clone, Copy)]
struct AxisFields {
    name: AxisName,
    min: WidgetId,
    max: WidgetId,
    label: WidgetId,
    scale: WidgetId,
}

/// The settings of an axis read from its fields.
#[derive(Debug)]
struct AxisPlan {
    name: AxisName,
    limits: (f64, f64),
    label: String,
    scale: &'static str,
}

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
    panels: WidgetId,
    title: WidgetId,
    /// Where the edited title sits.
    title_location: TitleLocation,
    axis_fields: [AxisFields; 2],
    legend: WidgetId,
    curves: Vec<CurveFields>,
    curve_pages: Option<WidgetId>,
    images: Vec<ImageFields>,
    image_pages: Option<WidgetId>,
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
        if tabs.len() > 1 {
            root.push(
                Tabs::new(tabs, move |tab| OptionsMessage::Tab { page, tab }),
                0.0,
            );
        }
        let mut panels = Stack::new();
        let (title_location, shown) = axes.shown_title();
        let (title, axis_fields, legend, general) = general_form(axes, &shown);
        panels.push(general);
        let (curves, curve_pages) = if lines.is_empty() {
            (Vec::new(), None)
        } else {
            let (curves, panel, pages) = curves_panel(&lines, page);
            panels.push(panel);
            (curves, Some(pages))
        };
        let (image_fields, image_pages) = if images.is_empty() {
            (Vec::new(), None)
        } else {
            let (fields, panel, pages) = images_panel(&images, page);
            panels.push(panel);
            (fields, Some(pages))
        };
        let panels = root.push(panels, 1.0);
        let page = Self {
            axes: axes.clone(),
            panels,
            title,
            title_location,
            axis_fields,
            legend,
            curves,
            curve_pages,
            images: image_fields,
            image_pages,
        };
        (page, root)
    }

    /// Shows tab `tab`.
    pub(super) fn show_tab(&self, window: &mut UiWindow<OptionsMessage>, tab: usize) {
        show(window, self.panels, tab);
    }

    /// Shows the fields of curve `index`.
    pub(super) fn show_curve(&self, window: &mut UiWindow<OptionsMessage>, index: usize) {
        if let Some(pages) = self.curve_pages {
            show(window, pages, index);
        }
    }

    /// Shows the fields of image `index`.
    pub(super) fn show_image(&self, window: &mut UiWindow<OptionsMessage>, index: usize) {
        if let Some(pages) = self.image_pages {
            show(window, pages, index);
        }
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
            let (min, max) = axis.limits;
            // An unknown scale cannot come from the fixed choices.
            match axis.name {
                AxisName::X => {
                    if ax.xscale() != axis.scale {
                        let _ = ax.set_xscale(axis.scale);
                    }
                    ax.set_xlim(min, max).set_xlabel(axis.label);
                }
                AxisName::Y => {
                    if ax.yscale() != axis.scale {
                        let _ = ax.set_yscale(axis.scale);
                    }
                    ax.set_ylim(min, max).set_ylabel(axis.label);
                }
            }
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

/// The title, both axes and the legend switch.
fn general_form(
    axes: &Axes,
    shown_title: &str,
) -> (
    WidgetId,
    [AxisFields; 2],
    WidgetId,
    plt_ui::Form<OptionsMessage>,
) {
    let mut form = Form::new();
    form.section("");
    let title = form.text("Title", shown_title);
    let mut axis = |name: AxisName| {
        let (heading, (min, max), scale) = match name {
            AxisName::X => ("X axis", axes.xlim(), axes.xscale()),
            AxisName::Y => ("Y axis", axes.ylim(), axes.yscale()),
        };
        form.section(heading);
        let (min, max) = form.pair(("Min", number_text(min)), ("Max", number_text(max)));
        let label = form.text("Label", axes.label(name));
        let current = SCALES.iter().position(|&s| s == scale).unwrap_or(0);
        let scale = form.choice("Scale", &SCALES, current);
        AxisFields {
            name,
            min,
            max,
            label,
            scale,
        }
    };
    let fields = [axis(AxisName::X), axis(AxisName::Y)];
    form.section("");
    let legend = form.wide(Checkbox::new("(Re-)Generate automatic legend", |_| {
        OptionsMessage::Edited
    }));
    (title, fields, legend, form.finish())
}

fn read_axis(
    window: &mut UiWindow<OptionsMessage>,
    fields: &AxisFields,
) -> Result<AxisPlan, FieldError> {
    let axis = match fields.name {
        AxisName::X => "X",
        AxisName::Y => "Y",
    };
    let min = parse_number(&format!("{axis} min"), &text_of(window, fields.min))?;
    let max = parse_number(&format!("{axis} max"), &text_of(window, fields.max))?;
    Ok(AxisPlan {
        name: fields.name,
        limits: (min, max),
        label: text_of(window, fields.label),
        scale: SCALES[chosen(window, fields.scale).min(SCALES.len() - 1)],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::lines::LineProps;

    #[test]
    fn fields_are_checked_then_applied() {
        use plt_platform::{EventLoop, WindowOptions};
        use plt_ui::TextInput;

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
        type_into(&mut window, page.title, "Edited");
        type_into(&mut window, page.axis_fields[0].max, "ten");
        let problem = page.read(&mut window).unwrap_err();
        assert_eq!(problem.0, "X max: \"ten\" is not a number");
        type_into(&mut window, page.axis_fields[0].max, "10");
        type_into(&mut window, page.curves[0].width, "3");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan), "the x limits changed");
        assert_eq!(ax.title(TitleLocation::Center), "Edited");
        assert_eq!(ax.xlim().1, 10.0);
        assert_eq!(line.linewidth(), 3.0);
    }

    #[test]
    fn the_title_is_edited_where_it_is_shown() {
        use plt_platform::{EventLoop, WindowOptions};
        use plt_ui::TextInput;

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
}
