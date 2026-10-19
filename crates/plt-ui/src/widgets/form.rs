//! Forms: labelled fields in rows sharing one aligned label column,
//! grouped into sections drawn as cards.

use std::fmt;
use std::ops::Range;

use plt_core::Point;

use crate::error::UiError;
use crate::geometry::{Constraints, Rect, Size};
use crate::text::{TextLayout, TextStyle};
use crate::theme::{Spacing, Theme};
use crate::widget::{LayoutCx, PaintCx, Widget, WidgetId, WidgetPod};

/// Space between the rows of a section.
const ROW_GAP: Spacing = Spacing::Sm;
/// Space between the label column and the fields.
const LABEL_GAP: Spacing = Spacing::Md;
/// Space between a card's outline and its rows.
const CARD_PADDING: Spacing = Spacing::Md;
/// Space between sections.
const SECTION_GAP: Spacing = Spacing::Lg;
/// Space between a section's title and its card.
const TITLE_GAP: Spacing = Spacing::Sm;

/// Text the form paints itself: a row's label or a section's title.
#[derive(Debug)]
struct Caption {
    text: String,
    /// The text laid out by the last measure.
    layout: Option<TextLayout>,
    /// Where the last arrangement put the text.
    origin: Point,
}

impl Caption {
    fn new(text: String) -> Self {
        Self {
            text,
            layout: None,
            origin: Point::ORIGIN,
        }
    }

    fn lay_out(&mut self, style: &TextStyle, scale_factor: f64) -> Result<Size, UiError> {
        let layout = TextLayout::new(&self.text, style, scale_factor)?;
        let size = layout.size();
        self.layout = Some(layout);
        Ok(size)
    }

    fn size(&self) -> Size {
        self.layout.as_ref().map_or(Size::ZERO, TextLayout::size)
    }
}

/// A row: a label and a field, or a field spanning both columns.
#[derive(Debug)]
struct Row {
    label: Option<Caption>,
    /// The row's height from the last measure.
    height: f64,
}

/// A run of rows, drawn on a card unless it is the untitled run before
/// the first section.
#[derive(Debug)]
struct Section {
    title: Option<Caption>,
    carded: bool,
    rows: Range<usize>,
    /// The card's rectangle from the last arrangement.
    card: Rect,
}

/// The form's spacing in pixels for one layout.
#[derive(Debug, Clone, Copy)]
struct Gaps {
    row: f64,
    label: f64,
    padding: f64,
    section: f64,
    title: f64,
}

impl Gaps {
    fn new(theme: &Theme, scale_factor: f64) -> Self {
        let px = |step| theme.space(step) * scale_factor;
        Self {
            row: px(ROW_GAP),
            label: px(LABEL_GAP),
            padding: px(CARD_PADDING),
            section: px(SECTION_GAP),
            title: px(TITLE_GAP),
        }
    }
}

/// Rows of labelled fields: the labels share one column, as wide as the
/// widest label (and at least [`Theme::label_min_width`]), and the fields
/// fill the rest of the width. Rows may also hold a field spanning both
/// columns, such as a check box.
///
/// Rows added after [`Form::section`] are grouped on a card under the
/// section's title; rows added before the first section have no card.
///
/// ```
/// use plt_ui::{Checkbox, Form, TextInput};
///
/// let mut form = Form::new();
/// form.section("X axis");
/// let label = form.row("Label", TextInput::new("time", |_| ()));
/// form.wide_row(Checkbox::new("Legend", |_| ()));
/// # let _ = label;
/// ```
pub struct Form<M> {
    fields: Vec<WidgetPod<M>>,
    /// One row per field.
    rows: Vec<Row>,
    sections: Vec<Section>,
    /// The label column's width from the last measure.
    label_width: f64,
}

impl<M> fmt::Debug for Form<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Form")
            .field("fields", &self.fields)
            .field("sections", &self.sections)
            .finish_non_exhaustive()
    }
}

impl<M: 'static> Default for Form<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static> Form<M> {
    /// An empty form.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            rows: Vec::new(),
            sections: vec![Section {
                title: None,
                carded: false,
                rows: 0..0,
                card: Rect::default(),
            }],
            label_width: 0.0,
        }
    }

    /// Starts a section: the rows added next are drawn on a card under
    /// `title` (no title when it is empty).
    pub fn section(&mut self, title: impl Into<String>) {
        let title = title.into();
        let start = self.rows.len();
        self.sections.push(Section {
            title: (!title.is_empty()).then(|| Caption::new(title)),
            carded: true,
            rows: start..start,
            card: Rect::default(),
        });
    }

    /// Adds a row with `label` beside `field`; returns the field's id.
    pub fn row(&mut self, label: impl Into<String>, field: impl Widget<M>) -> WidgetId {
        self.push(Some(Caption::new(label.into())), field)
    }

    /// Adds a row whose `field` spans the label column too; returns the
    /// field's id.
    pub fn wide_row(&mut self, field: impl Widget<M>) -> WidgetId {
        self.push(None, field)
    }

    fn push(&mut self, label: Option<Caption>, field: impl Widget<M>) -> WidgetId {
        let pod = WidgetPod::new(field);
        let id = pod.id();
        self.fields.push(pod);
        self.rows.push(Row { label, height: 0.0 });
        if let Some(section) = self.sections.last_mut() {
            section.rows.end = self.rows.len();
        }
        id
    }

    /// The sections with rows.
    fn shown_sections(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.sections.len()).filter(|&index| !self.sections[index].rows.is_empty())
    }

    /// The space between a section's card and its rows.
    fn padding(section: &Section, gaps: Gaps) -> f64 {
        if section.carded { gaps.padding } else { 0.0 }
    }

    /// The width the label of `row` and the gap after it take.
    fn label_column(&self, row: usize, gaps: Gaps) -> f64 {
        if self.rows[row].label.is_some() {
            self.label_width + gaps.label
        } else {
            0.0
        }
    }

    /// The width left for the field of `row` in a section `inner` pixels
    /// wide.
    fn field_width(&self, row: usize, inner: f64, gaps: Gaps) -> f64 {
        (inner - self.label_column(row, gaps)).max(0.0)
    }

    /// Lays out the labels and titles; returns the label column's width.
    fn lay_out_captions(&mut self, theme: &Theme, scale_factor: f64) -> Result<f64, UiError> {
        let style = theme.text_style();
        let mut widest: Option<f64> = None;
        for label in self.rows.iter_mut().filter_map(|row| row.label.as_mut()) {
            let width = label.lay_out(&style, scale_factor)?.width;
            widest = Some(widest.map_or(width, |w| w.max(width)));
        }
        let title_style = style.bold();
        for title in self.sections.iter_mut().filter_map(|s| s.title.as_mut()) {
            title.lay_out(&title_style, scale_factor)?;
        }
        Ok(widest.map_or(0.0, |w| w.max(theme.label_min_width * scale_factor)))
    }
}

impl<M: 'static> Widget<M> for Form<M> {
    fn measure(
        &mut self,
        cx: &mut LayoutCx<'_, M>,
        constraints: Constraints,
    ) -> Result<Size, UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let gaps = Gaps::new(theme, scale);
        self.label_width = self.lay_out_captions(theme, scale)?;
        let width = constraints.max.width;
        let bounded = width.is_finite();
        let mut natural_width: f64 = 0.0;
        let mut height = 0.0;
        let shown: Vec<usize> = self.shown_sections().collect();
        for (position, &index) in shown.iter().enumerate() {
            let section = &self.sections[index];
            if position > 0 {
                height += gaps.section;
            }
            if let Some(title) = &section.title {
                height += title.size().height + gaps.title;
            }
            let padding = Self::padding(section, gaps);
            let rows = section.rows.clone();
            height += 2.0 * padding + gaps.row * (rows.len() - 1) as f64;
            for row in rows {
                let field = if bounded {
                    let column = self.field_width(row, width - 2.0 * padding, gaps);
                    Constraints {
                        min: Size::new(column, 0.0),
                        max: Size::new(column, f64::INFINITY),
                    }
                } else {
                    Constraints::loose(Size::INFINITE)
                };
                let size = self.fields[row].measure(cx, field)?;
                let label_height = self.rows[row]
                    .label
                    .as_ref()
                    .map_or(0.0, |l| l.size().height);
                self.rows[row].height = size.height.max(label_height);
                height += self.rows[row].height;
                let used = 2.0 * padding + self.label_column(row, gaps) + size.width;
                natural_width = natural_width.max(used);
            }
        }
        let width = if bounded { width } else { natural_width };
        Ok(constraints.constrain(Size::new(width, height)))
    }

    fn arrange(&mut self, cx: &mut LayoutCx<'_, M>, rect: Rect) -> Result<(), UiError> {
        let gaps = Gaps::new(cx.theme(), cx.scale_factor());
        let mut y = rect.y;
        let shown: Vec<usize> = self.shown_sections().collect();
        for (position, &index) in shown.iter().enumerate() {
            if position > 0 {
                y += gaps.section;
            }
            let padding = Self::padding(&self.sections[index], gaps);
            let x = rect.x + padding;
            if let Some(title) = &mut self.sections[index].title {
                title.origin = Point::new(x, y);
                y += title.size().height + gaps.title;
            }
            let top = y;
            y += padding;
            let rows = self.sections[index].rows.clone();
            let inner = rect.width - 2.0 * padding;
            for row in rows.clone() {
                let width = self.field_width(row, inner, gaps);
                let height = self.rows[row].height;
                if let Some(label) = &mut self.rows[row].label {
                    label.origin = Point::new(x, y + (height - label.size().height) / 2.0);
                }
                let field = Rect::new(x + inner - width, y, width, height);
                self.fields[row].arrange(cx, field.snapped())?;
                y += height;
                if row + 1 < rows.end {
                    y += gaps.row;
                }
            }
            y += padding;
            self.sections[index].card = Rect::new(rect.x, top, rect.width, y - top).snapped();
        }
        Ok(())
    }

    fn paint(&self, cx: &mut PaintCx<'_>, _rect: Rect) -> Result<(), UiError> {
        let theme = cx.theme();
        let scale = cx.scale_factor();
        let radius = theme.card_radius * scale;
        for section in self.shown_sections().map(|index| &self.sections[index]) {
            if let Some(title) = &section.title
                && let Some(layout) = &title.layout
            {
                cx.canvas()
                    .draw_text(layout, title.origin, theme.text_secondary)?;
            }
            if section.carded {
                cx.canvas()
                    .fill_rounded_rect(section.card, radius, theme.surface)?;
                cx.canvas()
                    .stroke_rounded_rect(section.card, radius, scale, theme.separator)?;
            }
            for row in &self.rows[section.rows.clone()] {
                if let Some(label) = &row.label
                    && let Some(layout) = &label.layout
                {
                    cx.canvas().draw_text(layout, label.origin, theme.text)?;
                }
            }
        }
        Ok(())
    }

    fn children(&self) -> &[WidgetPod<M>] {
        &self.fields
    }

    fn children_mut(&mut self) -> &mut [WidgetPod<M>] {
        &mut self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::testing::Fixed;

    fn layout(form: &mut Form<()>, width: f64) -> Size {
        let theme = Theme::default();
        let mut messages = Vec::new();
        let mut cx = LayoutCx::for_tests(&theme, &mut messages, 1.0);
        let natural = form
            .measure(&mut cx, Constraints::loose(Size::INFINITE))
            .unwrap();
        let size = form
            .measure(
                &mut cx,
                Constraints::tight(Size::new(width, natural.height)),
            )
            .unwrap();
        form.arrange(&mut cx, Rect::from_origin_size(Point::ORIGIN, size))
            .unwrap();
        natural
    }

    #[test]
    fn labels_share_one_column() {
        let mut form = Form::new();
        let plain = form.row("Name", Fixed(Size::new(100.0, 28.0)));
        form.section("Limits");
        let short = form.row("Min", Fixed(Size::new(50.0, 28.0)));
        let wide = form.wide_row(Fixed(Size::new(30.0, 20.0)));
        let natural = layout(&mut form, 400.0);
        let rect = |id| {
            form.fields
                .iter()
                .find(|pod| pod.id() == id)
                .map(WidgetPod::rect)
                .unwrap()
        };
        let (plain, short, wide) = (rect(plain), rect(short), rect(wide));
        assert_eq!(plain.max_x(), 400.0, "fields fill the width");
        assert_eq!(short.max_x(), 400.0 - 12.0, "inside the card's padding");
        assert_eq!(
            plain.x,
            form.label_width + 12.0,
            "the label column is as wide as the widest label"
        );
        assert_eq!(short.x - 12.0, plain.x, "both columns line up");
        assert_eq!(wide.x, 12.0, "wide rows span the label column");
        assert!(form.label_width >= 64.0, "at least the theme's minimum");
        let card = form.sections[1].card;
        assert_eq!(card.max_y(), wide.max_y() + 12.0);
        assert!(card.y > plain.max_y() + 16.0, "the title sits between");
        assert_eq!(
            natural.width,
            form.label_width + 12.0 + 100.0,
            "the widest row"
        );
    }

    #[test]
    fn empty_sections_take_no_room() {
        let mut form: Form<()> = Form::new();
        form.section("Nothing here");
        form.section("");
        form.row("Only", Fixed(Size::new(10.0, 28.0)));
        let natural = layout(&mut form, 200.0);
        assert_eq!(natural.height, 28.0 + 2.0 * 12.0, "one untitled card");
        assert!(Form::<()>::new().shown_sections().next().is_none());
    }
}
