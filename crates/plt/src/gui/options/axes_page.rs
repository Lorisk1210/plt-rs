//! One axes in the figure options dialog: its titles, limits, labels and
//! scales, its curves and its images (matplotlib's `figure_edit` pages).

use plt_ui::{Checkbox, Choice, Flex, List, ListItem, Stack, UiWindow, WidgetId};

use super::OptionsMessage;
use super::fields::{
    FieldError, Form, chosen, hex, marker_text, number_text, parse_color, parse_marker,
    parse_number, text_of,
};
use crate::artist::ArtistHandle;
use crate::axes::{Axes, AxisName, TitleLocation};
use crate::colors::{Color, Colormap, colormaps};
use crate::image::{AxesImage, Interpolation};
use crate::legend::LegendOptions;
use crate::lines::{DrawStyle, Line2D, LineStyle};
use crate::markers::Marker;
use crate::rc::ColorOrAuto;

/// The scales offered, as matplotlib names them.
const SCALES: [&str; 4] = ["linear", "log", "symlog", "logit"];

/// The line styles offered.
const LINE_STYLES: [(&str, LineStyle); 5] = [
    ("Solid", LineStyle::Solid),
    ("Dashed", LineStyle::Dashed),
    ("DashDot", LineStyle::DashDot),
    ("Dotted", LineStyle::Dotted),
    ("None", LineStyle::None),
];

/// The draw styles offered.
const DRAW_STYLES: [(&str, DrawStyle); 4] = [
    ("Default", DrawStyle::Default),
    ("Steps (Pre)", DrawStyle::StepsPre),
    ("Steps (Mid)", DrawStyle::StepsMid),
    ("Steps (Post)", DrawStyle::StepsPost),
];

/// The fields of one axis.
#[derive(Debug, Clone, Copy)]
struct AxisFields {
    name: AxisName,
    min: WidgetId,
    max: WidgetId,
    label: WidgetId,
    scale: WidgetId,
}

/// The fields of one curve, and its marker code when the dialog opened.
#[derive(Debug)]
struct CurveFields {
    line: Line2D,
    label: WidgetId,
    style: WidgetId,
    draw_style: WidgetId,
    width: WidgetId,
    color: WidgetId,
    marker: WidgetId,
    marker_size: WidgetId,
    face: WidgetId,
    edge: WidgetId,
}

/// The fields of one image.
#[derive(Debug)]
struct ImageFields {
    image: AxesImage,
    label: WidgetId,
    cmap: WidgetId,
    min: WidgetId,
    max: WidgetId,
    interpolation: WidgetId,
}

/// The settings of an axis read from its fields.
#[derive(Debug)]
struct AxisPlan {
    name: AxisName,
    limits: (f64, f64),
    label: String,
    scale: &'static str,
}

/// The settings of a curve read from its fields.
#[derive(Debug)]
struct CurvePlan {
    label: String,
    style: Option<LineStyle>,
    draw_style: DrawStyle,
    width: f64,
    color: Color,
    marker: Option<(Marker, f64, Color, Color)>,
}

/// The settings of an image read from its fields.
#[derive(Debug)]
struct ImagePlan {
    label: String,
    cmap: Colormap,
    limits: Option<(f64, f64)>,
    interpolation: Interpolation,
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
    axis_fields: [AxisFields; 2],
    legend: WidgetId,
    curves: Vec<CurveFields>,
    curve_pages: Option<WidgetId>,
    images: Vec<ImageFields>,
    image_pages: Option<WidgetId>,
}

/// How an artist is named in the dialog's lists.
fn shown_label(label: &str) -> String {
    if label.is_empty() {
        "(no label)".to_owned()
    } else {
        label.to_owned()
    }
}

/// The curves the dialog edits: the lines not excluded from legends.
fn edited_lines(axes: &Axes) -> Vec<Line2D> {
    axes.lines()
        .into_iter()
        .filter(|line| line.label() != "_nolegend_")
        .collect()
}

impl AxesPage {
    /// The page for `axes`, number `page` of the dialog, and its widgets.
    pub(super) fn build(axes: &Axes, page: usize) -> (Self, Flex<OptionsMessage>) {
        let mut root = Flex::column().with_spacing(8.0);
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
                Choice::new(tabs, move |tab| OptionsMessage::Tab { page, tab }),
                0.0,
            );
        }
        let mut panels = Stack::new();
        let (title, axis_fields, legend, general) = general_form(axes);
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
        ax.set_title(plan.title);
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
            let image = &fields.image;
            image.set_label(plan.label);
            image
                .set_cmap(plan.cmap)
                .set_interpolation(plan.interpolation);
            if let Some((low, high)) = plan.limits {
                image.set_clim(Some(low.min(high)), Some(low.max(high)));
            }
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
fn general_form(axes: &Axes) -> (WidgetId, [AxisFields; 2], WidgetId, Flex<OptionsMessage>) {
    let mut form = Form::new();
    let title = form.text("Title", axes.title(TitleLocation::Center));
    let mut axis = |name: AxisName| {
        let (heading, (min, max), scale) = match name {
            AxisName::X => ("X axis", axes.xlim(), axes.xscale()),
            AxisName::Y => ("Y axis", axes.ylim(), axes.yscale()),
        };
        form.heading(heading);
        let min = form.text("Min", number_text(min));
        let max = form.text("Max", number_text(max));
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
    let legend = form.push(Checkbox::new("(Re-)Generate automatic legend", |_| {
        OptionsMessage::Edited
    }));
    (title, fields, legend, form.finish())
}

/// A list of curves beside the fields of the selected one.
fn curves_panel(
    lines: &[Line2D],
    page: usize,
) -> (Vec<CurveFields>, Flex<OptionsMessage>, WidgetId) {
    let mut panel = Flex::row().with_spacing(8.0);
    let items = lines
        .iter()
        .map(|line| ListItem::new(shown_label(&line.label())))
        .collect();
    let mut list = List::new(items, move |index| OptionsMessage::Curve { page, index });
    list.set_selected(Some(0));
    panel.push(list, 1.0);
    let mut pages = Stack::new();
    let fields = lines
        .iter()
        .map(|line| {
            let (fields, form) = curve_form(line);
            pages.push(form);
            fields
        })
        .collect();
    let pages = panel.push(pages, 3.0);
    (fields, panel, pages)
}

/// The fields of one curve.
fn curve_form(line: &Line2D) -> (CurveFields, Flex<OptionsMessage>) {
    let mut form = Form::new();
    let label = form.text("Label", line.label());
    form.heading("Line");
    let current = line.linestyle();
    let mut styles: Vec<&str> = LINE_STYLES.iter().map(|(name, _)| *name).collect();
    let style_index = LINE_STYLES
        .iter()
        .position(|(_, style)| *style == current)
        .unwrap_or_else(|| {
            styles.push("Custom");
            LINE_STYLES.len()
        });
    let style = form.choice("Line style", &styles, style_index);
    let draw = match line.drawstyle() {
        DrawStyle::Steps => DrawStyle::StepsPre,
        other => other,
    };
    let draw_index = DRAW_STYLES
        .iter()
        .position(|(_, d)| *d == draw)
        .unwrap_or(0);
    let names: Vec<&str> = DRAW_STYLES.iter().map(|(name, _)| *name).collect();
    let draw_style = form.choice("Draw style", &names, draw_index);
    let width = form.text("Width", number_text(line.linewidth()));
    let color = form.text("Color (RGBA)", line.rgba().map(hex).unwrap_or_default());
    form.heading("Marker");
    let marker = form.text("Style", marker_text(&line.marker()));
    let marker_size = form.text("Size", number_text(line.markersize()));
    let face = form.text(
        "Face color (RGBA)",
        marker_color(line, line.markerfacecolor()),
    );
    let edge = form.text(
        "Edge color (RGBA)",
        hex(line
            .effective_markeredgecolor()
            .resolve()
            .unwrap_or_default()),
    );
    let fields = CurveFields {
        line: line.clone(),
        label,
        style,
        draw_style,
        width,
        color,
        marker,
        marker_size,
        face,
        edge,
    };
    (fields, form.finish())
}

/// The hex text of a marker colour, following the line's colour when
/// automatic.
fn marker_color(line: &Line2D, color: ColorOrAuto) -> String {
    match color {
        ColorOrAuto::Color(color) => color.resolve().map(hex).unwrap_or_default(),
        ColorOrAuto::Auto => line.rgba().map(hex).unwrap_or_default(),
    }
}

/// A list of images beside the fields of the selected one.
fn images_panel(
    images: &[AxesImage],
    page: usize,
) -> (Vec<ImageFields>, Flex<OptionsMessage>, WidgetId) {
    let mut panel = Flex::row().with_spacing(8.0);
    let items = images
        .iter()
        .map(|image| ListItem::new(shown_label(&image.label())))
        .collect();
    let mut list = List::new(items, move |index| OptionsMessage::Image { page, index });
    list.set_selected(Some(0));
    panel.push(list, 1.0);
    let mut pages = Stack::new();
    let fields = images
        .iter()
        .map(|image| {
            let mut form = Form::new();
            let (low, high) = image.clim();
            let fields = ImageFields {
                image: image.clone(),
                label: form.text("Label", image.label()),
                cmap: form.text("Colormap", image.cmap().name()),
                min: form.text("Min. value", low.map(number_text).unwrap_or_default()),
                max: form.text("Max. value", high.map(number_text).unwrap_or_default()),
                interpolation: form.text("Interpolation", image.interpolation().name()),
            };
            pages.push(form.finish());
            fields
        })
        .collect();
    let pages = panel.push(pages, 3.0);
    (fields, panel, pages)
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

fn read_curve(
    window: &mut UiWindow<OptionsMessage>,
    fields: &CurveFields,
) -> Result<CurvePlan, FieldError> {
    let label = text_of(window, fields.label);
    let style = LINE_STYLES
        .get(chosen(window, fields.style))
        .map(|(_, style)| style.clone());
    let draw_style = DRAW_STYLES[chosen(window, fields.draw_style).min(DRAW_STYLES.len() - 1)].1;
    let width = parse_number(&format!("{label}: width"), &text_of(window, fields.width))?;
    let color = parse_color(&format!("{label}: color"), &text_of(window, fields.color))?;
    let marker_text = text_of(window, fields.marker);
    let marker = if marker_text.trim().eq_ignore_ascii_case("none") {
        None
    } else {
        Some((
            parse_marker(&format!("{label}: marker"), &marker_text)?,
            parse_number(
                &format!("{label}: marker size"),
                &text_of(window, fields.marker_size),
            )?,
            parse_color(
                &format!("{label}: face color"),
                &text_of(window, fields.face),
            )?,
            parse_color(
                &format!("{label}: edge color"),
                &text_of(window, fields.edge),
            )?,
        ))
    };
    Ok(CurvePlan {
        label,
        style,
        draw_style,
        width,
        color,
        marker,
    })
}

fn read_image(
    window: &mut UiWindow<OptionsMessage>,
    fields: &ImageFields,
) -> Result<ImagePlan, FieldError> {
    let label = text_of(window, fields.label);
    let name = text_of(window, fields.cmap);
    let cmap = colormaps::get(name.trim())
        .map_err(|_| FieldError(format!("{label}: no colormap is called {name:?}")))?;
    let (low, high) = (text_of(window, fields.min), text_of(window, fields.max));
    let limits = if low.trim().is_empty() && high.trim().is_empty() {
        None
    } else {
        Some((
            parse_number(&format!("{label}: min. value"), &low)?,
            parse_number(&format!("{label}: max. value"), &high)?,
        ))
    };
    let text = text_of(window, fields.interpolation);
    let interpolation = text
        .trim()
        .parse()
        .map_err(|_| FieldError(format!("{label}: {text:?} is not an interpolation")))?;
    Ok(ImagePlan {
        label,
        cmap,
        limits,
        interpolation,
    })
}

/// matplotlib's curve settings of `apply_callback`.
fn apply_curve(line: &Line2D, plan: CurvePlan) {
    line.set_label(plan.label);
    if let Some(style) = plan.style {
        line.set_linestyle(style);
    }
    line.set_drawstyle(plan.draw_style)
        .set_linewidth(plan.width)
        .set_alpha(None);
    line.set_color(plan.color);
    if let Some((marker, size, face, edge)) = plan.marker {
        line.set_marker(marker)
            .set_markersize(size)
            .set_markerfacecolor(ColorOrAuto::Color(face))
            .set_markeredgecolor(ColorOrAuto::Color(edge));
    }
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
    fn every_line_but_hidden_ones_is_edited() {
        let ax = Figure::new().subplot();
        ax.plot_with(
            vec![0.0, 1.0],
            vec![0.0, 1.0],
            LineProps::default().label("a"),
        )
        .unwrap();
        ax.plot(vec![0.0, 1.0], vec![1.0, 0.0]).unwrap();
        ax.plot_with(
            vec![0.0, 1.0],
            vec![1.0, 1.0],
            LineProps::default().label("_nolegend_"),
        )
        .unwrap();
        let labels: Vec<String> = edited_lines(&ax)
            .iter()
            .map(|l| shown_label(&l.label()))
            .collect();
        assert_eq!(labels, ["a", "(no label)"]);
    }
}
