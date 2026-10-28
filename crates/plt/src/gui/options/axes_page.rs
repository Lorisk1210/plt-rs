//! One axes in the figure options dialog: its titles, limits, labels and
//! scales, its curves and its images (matplotlib's `figure_edit` pages).

use plt_ui::{Checkbox, Flex, Spacing, Stack, Tabs, UiWindow, WidgetId};

use super::OptionsMessage;
use super::artists::{
    CurveFields, CurvePlan, Detail, ImageFields, ImagePlan, apply_curve, apply_image, curves_panel,
    edited_lines, images_panel, read_curve, read_image,
};
use super::fields::{
    FieldError, Form, chosen, date_text, date_zone, limit_text, parse_date, parse_limit,
    read_field, text_of,
};
use crate::axes::{Axes, AxisName, TitleLocation};
use crate::dates::TimeZone;
use crate::legend::LegendOptions;
use crate::scale::ScaleName;
use crate::units::UnitConverter;

/// The scales offered, as matplotlib names them.
static SCALES: [ScaleName; 4] = [
    ScaleName::Linear,
    ScaleName::Log,
    ScaleName::Symlog,
    ScaleName::Logit,
];

/// The field of an axis limit, showing it rounded, or as a date on an
/// axis of dates (as matplotlib's editor shows them).
#[derive(Debug, Clone)]
struct LimitField {
    id: WidgetId,
    /// The limit, which the field keeps while its text is the one shown.
    value: f64,
    /// The text the field was given.
    shown: String,
    /// On an axis of dates, the zone the limit, a date number, is shown
    /// and typed in.
    zone: Option<TimeZone>,
}

impl LimitField {
    /// The text showing limit `value` on an axis with `resolution` (see
    /// [`limit_text`]), as a date in `zone` on an axis of dates.
    fn text(value: f64, resolution: Option<f64>, zone: Option<&TimeZone>) -> String {
        zone.and_then(|zone| date_text(value, zone))
            .unwrap_or_else(|| limit_text(value, resolution))
    }

    /// The limit typed into the field called `name`, or the exact limit
    /// it showed if the text was not edited (so that applying unedited
    /// fields changes no limit).
    fn read(&self, window: &mut UiWindow<OptionsMessage>, name: &str) -> Result<f64, FieldError> {
        read_field(window, self.id, |text| {
            if text == self.shown {
                Ok(self.value)
            } else if let Some(zone) = &self.zone {
                parse_date(name, text, zone)
            } else {
                parse_limit(name, text)
            }
        })
    }
}

/// The fields of one axis.
#[derive(Debug, Clone)]
struct AxisFields {
    name: AxisName,
    min: LimitField,
    max: LimitField,
    label: WidgetId,
    scale: WidgetId,
}

/// The settings of an axis read from its fields.
#[derive(Debug)]
struct AxisPlan {
    name: AxisName,
    limits: (f64, f64),
    label: String,
    scale: ScaleName,
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
        let mut general = std::iter::once(self.title).chain(
            self.axis_fields
                .iter()
                .flat_map(|axis| [axis.min.id, axis.max.id, axis.label]),
        );
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
            let (min, max) = axis.limits;
            // An unknown scale cannot come from the fixed choices.
            match axis.name {
                AxisName::X => {
                    if ax.xscale() != axis.scale {
                        let _ = ax.set_xscale(axis.scale);
                    }
                    ax.set_xlim(min, max)
                        .expect("the form takes finite limits only")
                        .set_xlabel(axis.label);
                }
                AxisName::Y => {
                    if ax.yscale() != axis.scale {
                        let _ = ax.set_yscale(axis.scale);
                    }
                    ax.set_ylim(min, max)
                        .expect("the form takes finite limits only")
                        .set_ylabel(axis.label);
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
        let (heading, (min, max), scale, axis) = match name {
            AxisName::X => ("X axis", axes.xlim(), axes.xscale(), axes.xaxis()),
            AxisName::Y => ("Y axis", axes.ylim(), axes.yscale(), axes.yaxis()),
        };
        form.section(heading);
        let zone = (axis.converter().as_ref())
            .is_some_and(UnitConverter::is_dates)
            .then(date_zone);
        let shown = |value| {
            let resolution = axes.pixel_resolution(name, value);
            LimitField::text(value, resolution, zone.as_ref())
        };
        let (min_text, max_text) = (shown(min), shown(max));
        let (min_id, max_id) = form.pair(("Min", min_text.clone()), ("Max", max_text.clone()));
        let min = LimitField {
            id: min_id,
            value: min,
            shown: min_text,
            zone: zone.clone(),
        };
        let max = LimitField {
            id: max_id,
            value: max,
            shown: max_text,
            zone,
        };
        let label = form.text("Label", axes.label(name));
        let current = SCALES.iter().position(|s| *s == scale).unwrap_or(0);
        let scale = form.choice("Scale", &SCALES.each_ref().map(ScaleName::as_str), current);
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
    let min = fields.min.read(window, &format!("{axis} min"))?;
    let max = fields.max.read(window, &format!("{axis} max"))?;
    Ok(AxisPlan {
        name: fields.name,
        limits: (min, max),
        label: text_of(window, fields.label),
        scale: SCALES[chosen(window, fields.scale).min(SCALES.len() - 1)].clone(),
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
        let x_max = page.axis_fields[0].max.id;
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
        use plt_platform::{EventLoop, WindowOptions};
        use plt_ui::TextInput;

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
        assert_eq!(text_of(&mut window, x.min.id), "-0.49");
        assert_eq!(text_of(&mut window, x.max.id), "10.29");
        assert_eq!(text_of(&mut window, y.max.id), "1.05");
        let plan = page.read(&mut window).unwrap();
        assert!(!page.apply(plan), "no limit changed");
        assert_eq!((ax.xlim(), ax.ylim()), ((x0, x1), ylim));
        window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(x.max.id)
            .unwrap()
            .set_text("12");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan));
        assert_eq!(ax.xlim(), (x0, 12.0), "the unedited minimum stays exact");
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

    #[test]
    fn limits_of_dates_show_and_read_as_dates() {
        use plt_platform::{EventLoop, WindowOptions};
        use plt_ui::TextInput;

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
        assert_eq!(text_of(&mut window, x.min.id), "2020-01-01 00:00:00");
        assert_eq!(text_of(&mut window, x.max.id), "2020-01-03 12:00:00");
        assert!(!text_of(&mut window, y.max.id).contains('-'), "numbers");
        window
            .ui_mut()
            .widget_mut::<TextInput<OptionsMessage>>(x.max.id)
            .unwrap()
            .set_text("2020-01-05");
        let plan = page.read(&mut window).unwrap();
        assert!(page.apply(plan));
        assert_eq!(ax.xlim(), (18_262.0, 18_266.0));
    }

    #[test]
    fn limits_of_dates_show_and_read_in_the_timezone_rc_param() {
        use plt_platform::{EventLoop, WindowOptions};
        use plt_ui::TextInput;

        use crate::dates::DateTime;

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
            assert_eq!(text_of(&mut window, x.min.id), "2020-01-01 09:00:00");
            assert_eq!(text_of(&mut window, x.max.id), "2020-01-03 21:00:00");
            window
                .ui_mut()
                .widget_mut::<TextInput<OptionsMessage>>(x.max.id)
                .unwrap()
                .set_text("2020-01-05 09:00");
            let plan = page.read(&mut window).unwrap();
            assert!(page.apply(plan));
            assert_eq!(ax.xlim(), (18_262.0, 18_266.0), "typed in Tokyo time");
        });
    }

    #[test]
    fn a_refused_field_is_shown_on_its_axes_tab_and_curve() {
        use plt_platform::EventLoop;
        use plt_ui::{Dropdown, TextInput};

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
