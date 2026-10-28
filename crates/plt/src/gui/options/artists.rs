//! The curves and images of an axes in the figure options dialog: their
//! fields, how the fields are read, and how the settings are applied.

use plt_ui::{Flex, List, ListItem, Spacing, Stack, UiWindow, WidgetId};

use super::OptionsMessage;
use super::fields::{
    FieldError, Form, chosen, marker_text, number_text, parse_color, parse_marker, parse_number,
    read_field, text_of,
};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::colors::{Color, Colormap, colormaps};
use crate::image::{AxesImage, Interpolation};
use crate::lines::{DrawStyle, Line2D, LineStyle};
use crate::markers::Marker;
use crate::rc::ColorOrAuto;

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

/// The fields of one curve, and its marker code when the dialog opened.
#[derive(Debug)]
pub(super) struct CurveFields {
    pub(super) line: Line2D,
    pub(super) label: WidgetId,
    pub(super) style: WidgetId,
    pub(super) draw_style: WidgetId,
    pub(super) width: WidgetId,
    pub(super) color: WidgetId,
    pub(super) marker: WidgetId,
    pub(super) marker_size: WidgetId,
    pub(super) face: WidgetId,
    pub(super) edge: WidgetId,
}

impl CurveFields {
    /// The text fields, which a problem may name.
    pub(super) fn text_fields(&self) -> [WidgetId; 7] {
        [
            self.label,
            self.width,
            self.color,
            self.marker,
            self.marker_size,
            self.face,
            self.edge,
        ]
    }
}

/// The fields of one image.
#[derive(Debug)]
pub(super) struct ImageFields {
    pub(super) image: AxesImage,
    pub(super) label: WidgetId,
    pub(super) cmap: WidgetId,
    pub(super) min: WidgetId,
    pub(super) max: WidgetId,
    pub(super) interpolation: WidgetId,
}

impl ImageFields {
    /// The text fields, which a problem may name.
    pub(super) fn text_fields(&self) -> [WidgetId; 5] {
        [
            self.label,
            self.cmap,
            self.min,
            self.max,
            self.interpolation,
        ]
    }
}

/// The list of an axes' curves or images, and the stack of pages beside
/// it holding the fields of each.
#[derive(Debug, Clone, Copy)]
pub(super) struct Detail {
    list: WidgetId,
    pages: WidgetId,
}

impl Detail {
    /// Selects artist `index` in the list and shows its fields.
    pub(super) fn show(self, window: &mut UiWindow<OptionsMessage>, index: usize) {
        if let Some(list) = window
            .ui_mut()
            .widget_mut::<List<OptionsMessage>>(self.list)
        {
            list.set_selected(Some(index));
        }
        if let Some(pages) = window
            .ui_mut()
            .widget_mut::<Stack<OptionsMessage>>(self.pages)
        {
            pages.show(index);
        }
    }
}

/// The settings of a curve read from its fields.
#[derive(Debug)]
pub(super) struct CurvePlan {
    label: String,
    style: Option<LineStyle>,
    draw_style: DrawStyle,
    width: f64,
    color: Color,
    marker: Option<(Marker, f64, Color, Color)>,
}

/// The settings of an image read from its fields.
#[derive(Debug)]
pub(super) struct ImagePlan {
    label: String,
    cmap: Colormap,
    limits: Option<(f64, f64)>,
    interpolation: Interpolation,
}

/// How an artist is named in the dialog's lists.
pub(super) fn shown_label(label: &str) -> String {
    if label.is_empty() {
        "(no label)".to_owned()
    } else {
        label.to_owned()
    }
}

/// The curves the dialog edits: the lines not excluded from legends.
pub(super) fn edited_lines(axes: &Axes) -> Vec<Line2D> {
    axes.lines()
        .into_iter()
        .filter(|line| line.label() != "_nolegend_")
        .collect()
}

/// A list of an axes' artists named `names` beside a stack of pages, the
/// fields of the selected artist: the list sends `message(index)` when
/// another is selected. Returns the panel and its parts.
fn master_detail(
    names: Vec<ListItem>,
    message: impl Fn(usize) -> OptionsMessage + 'static,
    pages: Stack<OptionsMessage>,
) -> (Flex<OptionsMessage>, Detail) {
    let mut panel = Flex::row().with_spacing(Spacing::Lg);
    let mut list = List::new(names, message);
    list.set_selected(Some(0));
    let list = panel.push(list, 0.0);
    let pages = panel.push(pages, 1.0);
    (panel, Detail { list, pages })
}

/// A list of curves beside the fields of the selected one.
pub(super) fn curves_panel(
    lines: &[Line2D],
    page: usize,
) -> (Vec<CurveFields>, Flex<OptionsMessage>, Detail) {
    let items = lines
        .iter()
        .map(|line| ListItem::new(shown_label(&line.label())))
        .collect();
    let mut pages = Stack::new();
    let fields = lines
        .iter()
        .map(|line| {
            let (fields, form) = curve_form(line);
            pages.push(form);
            fields
        })
        .collect();
    let message = move |index| OptionsMessage::Curve { page, index };
    let (panel, detail) = master_detail(items, message, pages);
    (fields, panel, detail)
}

/// The fields of one curve.
fn curve_form(line: &Line2D) -> (CurveFields, plt_ui::Form<OptionsMessage>) {
    let mut form = Form::new();
    form.section("");
    let label = form.text("Label", line.label());
    form.section("Line");
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
    let color = form.color(
        "Color (RGBA)",
        line.rgba().map(|c| c.to_hex(true)).unwrap_or_default(),
    );
    form.section("Marker");
    let (marker, marker_size) = form.pair(
        ("Style", marker_text(&line.marker())),
        ("Size", number_text(line.markersize())),
    );
    let face = form.color(
        "Face color (RGBA)",
        marker_color(line, line.markerfacecolor()),
    );
    let edge = form.color(
        "Edge color (RGBA)",
        line.effective_markeredgecolor()
            .resolve()
            .unwrap_or_default()
            .to_hex(true),
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
        ColorOrAuto::Color(color) => color.resolve().map(|c| c.to_hex(true)).unwrap_or_default(),
        ColorOrAuto::Auto => line.rgba().map(|c| c.to_hex(true)).unwrap_or_default(),
    }
}

/// A list of images beside the fields of the selected one.
pub(super) fn images_panel(
    images: &[AxesImage],
    page: usize,
) -> (Vec<ImageFields>, Flex<OptionsMessage>, Detail) {
    let items = images
        .iter()
        .map(|image| ListItem::new(shown_label(&image.label())))
        .collect();
    let mut pages = Stack::new();
    let fields = images
        .iter()
        .map(|image| {
            let (fields, form) = image_form(image);
            pages.push(form);
            fields
        })
        .collect();
    let message = move |index| OptionsMessage::Image { page, index };
    let (panel, detail) = master_detail(items, message, pages);
    (fields, panel, detail)
}

/// The fields of one image.
fn image_form(image: &AxesImage) -> (ImageFields, plt_ui::Form<OptionsMessage>) {
    let mut form = Form::new();
    form.section("");
    let label = form.text("Label", image.label());
    form.section("Image");
    let cmap = form.text("Colormap", image.cmap().name());
    let (low, high) = image.clim();
    let (min, max) = form.pair(
        ("Min. value", low.map(number_text).unwrap_or_default()),
        ("Max. value", high.map(number_text).unwrap_or_default()),
    );
    let interpolation = form.text("Interpolation", image.interpolation().name());
    let fields = ImageFields {
        image: image.clone(),
        label,
        cmap,
        min,
        max,
        interpolation,
    };
    (fields, form.finish())
}

pub(super) fn read_curve(
    window: &mut UiWindow<OptionsMessage>,
    fields: &CurveFields,
) -> Result<CurvePlan, FieldError> {
    let label = text_of(window, fields.label);
    let style = LINE_STYLES
        .get(chosen(window, fields.style))
        .map(|(_, style)| style.clone());
    let draw_style = DRAW_STYLES[chosen(window, fields.draw_style).min(DRAW_STYLES.len() - 1)].1;
    let name = |field: &str| format!("{label}: {field}");
    let width = read_field(window, fields.width, |t| parse_number(&name("width"), t))?;
    let color = read_field(window, fields.color, |t| parse_color(&name("color"), t))?;
    let marker = if text_of(window, fields.marker)
        .trim()
        .eq_ignore_ascii_case("none")
    {
        None
    } else {
        Some((
            read_field(window, fields.marker, |t| parse_marker(&name("marker"), t))?,
            read_field(window, fields.marker_size, |t| {
                parse_number(&name("marker size"), t)
            })?,
            read_field(window, fields.face, |t| parse_color(&name("face color"), t))?,
            read_field(window, fields.edge, |t| parse_color(&name("edge color"), t))?,
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

pub(super) fn read_image(
    window: &mut UiWindow<OptionsMessage>,
    fields: &ImageFields,
) -> Result<ImagePlan, FieldError> {
    let label = text_of(window, fields.label);
    let cmap = read_field(window, fields.cmap, |name| {
        colormaps::get(name.trim()).map_err(|_| format!("{label}: no colormap is called {name:?}"))
    })?;
    let (low, high) = (text_of(window, fields.min), text_of(window, fields.max));
    let limits = if low.trim().is_empty() && high.trim().is_empty() {
        None
    } else {
        let name = |field: &str| format!("{label}: {field}");
        Some((
            read_field(window, fields.min, |t| parse_number(&name("min. value"), t))?,
            read_field(window, fields.max, |t| parse_number(&name("max. value"), t))?,
        ))
    };
    let interpolation = read_field(window, fields.interpolation, |text| {
        text.trim()
            .parse()
            .map_err(|_| format!("{label}: {text:?} is not an interpolation"))
    })?;
    Ok(ImagePlan {
        label,
        cmap,
        limits,
        interpolation,
    })
}

/// matplotlib's curve settings of `apply_callback`.
pub(super) fn apply_curve(line: &Line2D, plan: CurvePlan) {
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

/// matplotlib's image settings of `apply_callback`.
pub(super) fn apply_image(image: &AxesImage, plan: ImagePlan) {
    image.set_label(plan.label);
    image
        .set_cmap(plan.cmap)
        .set_interpolation(plan.interpolation);
    if let Some((low, high)) = plan.limits {
        image.set_clim(Some(low.min(high)), Some(low.max(high)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;
    use crate::lines::LineProps;

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
