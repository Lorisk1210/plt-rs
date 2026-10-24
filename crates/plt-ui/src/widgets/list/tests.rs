//! Tests of lists: rows, scrolling and keys.

use super::*;

fn list(rows: usize) -> List<usize> {
    let items = (0..rows)
        .map(|i| ListItem::new(format!("row {i}")))
        .collect();
    let mut list = List::new(items, |row| row);
    list.metrics = Metrics {
        row_height: 10.0,
        inset: 0.0,
        padding: 0.0,
        viewport: 50.0,
        scrollbar_width: 6.0,
        min_thumb: 20.0,
    };
    list
}

#[test]
fn rows_under_the_pointer_and_scrolling() {
    let mut list = list(20);
    assert_eq!(list.row_at(25.0), Some(2));
    assert_eq!(list.row_at(55.0), None, "below the visible rows");
    assert_eq!(list.max_scroll(), 150.0);
    list.set_selected(Some(9));
    assert_eq!(list.scroll, 50.0, "the selection scrolls into view");
    assert_eq!(list.row_at(0.0), Some(5));
    list.set_selected(Some(99));
    assert_eq!(list.selected(), None);
    list.set_items(vec![ListItem::new("only")]);
    assert_eq!((list.items().len(), list.scroll), (1, 0.0));
    assert_eq!(list.row_at(15.0), None);
}

#[test]
fn the_scroll_bar_shows_where_the_rows_are() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    assert!(list(5).scroll_bar(rect).is_none(), "all rows fit");
    let mut long = list(100);
    let (track, top) = long.scroll_bar(rect).unwrap();
    assert_eq!((track.x, track.height), (94.0, 50.0));
    assert_eq!(top.y, 0.0);
    assert_eq!(top.height, 20.0, "never shorter than the minimum");
    long.scroll = long.max_scroll();
    let (_, bottom) = long.scroll_bar(rect).unwrap();
    assert_eq!(bottom.max_y(), track.max_y());
}

fn press(key: Key, text: Option<&str>) -> UiEvent {
    UiEvent::Key(KeyEvent {
        key,
        state: ButtonState::Pressed,
        repeat: false,
        text: text.map(str::to_owned),
        modifiers: plt_platform::Modifiers::NONE,
        scancode: 0,
    })
}

#[test]
fn keys_move_the_selection_into_view() {
    let mut long = list(20);
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut messages = Vec::new();
    let mut cx = EventCx::new(&mut messages);
    long.event(&mut cx, &press(Key::Up, None), rect);
    assert_eq!(
        long.selected(),
        Some(0),
        "the first key selects the first row"
    );
    long.event(&mut cx, &press(Key::PageDown, None), rect);
    assert_eq!(long.selected(), Some(4), "five rows shown: a page is four");
    long.event(&mut cx, &press(Key::PageDown, None), rect);
    assert_eq!((long.selected(), long.scroll), (Some(8), 40.0));
    long.event(&mut cx, &press(Key::End, None), rect);
    assert_eq!((long.selected(), long.scroll), (Some(19), 150.0));
    long.event(&mut cx, &press(Key::PageUp, None), rect);
    long.event(&mut cx, &press(Key::Home, None), rect);
    assert_eq!((long.selected(), long.scroll), (Some(0), 0.0));
    long.event(&mut cx, &press(Key::Character('r'), Some("r")), rect);
    long.event(&mut cx, &press(Key::Character('o'), Some("o")), rect);
    long.event(&mut cx, &press(Key::Character('w'), Some("w")), rect);
    long.event(&mut cx, &press(Key::Character(' '), Some(" ")), rect);
    long.event(&mut cx, &press(Key::Character('7'), Some("7")), rect);
    assert_eq!(long.selected(), Some(7), "typed 'row 7'");
    assert!(cx.handled);
    assert_eq!(messages, [0, 4, 8, 19, 15, 0, 1, 7]);
}

#[test]
fn enter_activates_only_lists_with_rows_to_activate() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut plain = list(3);
    plain.set_selected(Some(1));
    let mut messages = Vec::new();
    let mut cx = EventCx::new(&mut messages);
    plain.event(&mut cx, &press(Key::Enter, None), rect);
    assert!(!cx.handled, "Enter goes on to the default button");
    let mut opening = list(3).on_activate(|row| row + 100);
    opening.event(&mut cx, &press(Key::Enter, None), rect);
    assert!(!cx.handled, "nothing selected");
    opening.set_selected(Some(2));
    opening.event(&mut cx, &press(Key::Enter, None), rect);
    assert!(cx.handled);
    assert_eq!(messages, [102]);
}

#[test]
fn dragging_the_thumb_scrolls() {
    let mut long = list(100);
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut messages = Vec::new();
    let mut cx = EventCx::new(&mut messages);
    let press = |y| UiEvent::PointerDown {
        button: MouseButton::Left,
        position: Point::new(97.0, y),
        modifiers: plt_platform::Modifiers::NONE,
        click_count: 1,
    };
    long.event(&mut cx, &press(5.0), rect);
    let drag = UiEvent::PointerMove {
        position: Point::new(97.0, 35.0),
        modifiers: plt_platform::Modifiers::NONE,
    };
    long.event(&mut cx, &drag, rect);
    assert_eq!(long.scroll, long.max_scroll(), "dragged to the bottom");
    long.event(&mut cx, &press(0.0), rect);
    assert!(long.scroll < 50.0, "a press above the thumb pages up");
    assert!(messages.is_empty(), "the scroll bar selects nothing");
}
