use std::cell::Cell;

use super::*;

fn spec(name: &str, kind: ToolKind) -> ToolSpec {
    ToolSpec {
        name: name.into(),
        description: format!("the {name} tool"),
        keys: vec![format!("ctrl+{name}")],
        kind,
        ..ToolSpec::default()
    }
}

/// A toggle of radio group `group`, if any, logging its uses to `log`.
fn radio(
    fig: &Figure,
    name: &str,
    group: Option<&str>,
    log: &Rc<RefCell<Vec<(String, bool)>>>,
) -> Result<(), ToolError> {
    let spec = ToolSpec {
        radio_group: group.map(str::to_owned),
        ..spec(name, ToolKind::Toggle)
    };
    let log = Rc::clone(log);
    fig.add_tool(spec, move |event| {
        log.borrow_mut()
            .push((event.name.to_owned(), event.toggled));
    })
    .map(|_| ())
}

#[test]
fn triggers_run_and_toggles_switch() {
    let fig = Figure::new();
    let uses = Rc::new(Cell::new(0));
    let count = Rc::clone(&uses);
    fig.add_tool(spec("a", ToolKind::Trigger), move |_| {
        count.set(count.get() + 1);
    })
    .unwrap();
    let states = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&states);
    fig.add_tool(spec("b", ToolKind::Toggle), move |event| {
        log.borrow_mut().push(event.toggled);
        // Callbacks may use the figure.
        assert_eq!(event.figure.tool_names(), ["a", "b"]);
    })
    .unwrap();
    assert!(fig.trigger_tool("a") && fig.trigger_tool("a"));
    assert!(fig.trigger_tool("b") && fig.trigger_tool("b"));
    assert_eq!(uses.get(), 2);
    assert_eq!(*states.borrow(), [true, false]);
    assert_eq!(fig.tool_toggled("a"), Some(false));
    assert!(!fig.trigger_tool("missing"));
    assert_eq!(fig.tool_for_key("ctrl+b").as_deref(), Some("b"));
}

#[test]
fn names_are_unique_and_tools_removable() {
    let fig = Figure::new();
    fig.add_tool(spec("a", ToolKind::Trigger), |_| {}).unwrap();
    assert_eq!(
        fig.add_tool(spec("a", ToolKind::Toggle), |_| {})
            .unwrap_err(),
        ToolError::Duplicate("a".into())
    );
    assert_eq!(
        fig.add_tool(spec("", ToolKind::Toggle), |_| {})
            .unwrap_err(),
        ToolError::EmptyName
    );
    assert!(fig.remove_tool("a") && !fig.remove_tool("a"));
    assert_eq!(fig.tool_names(), Vec::<String>::new());
}

#[test]
fn radio_groups_switch_one_toggle_at_a_time() {
    let fig = Figure::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    for (name, group) in [("a", Some("g")), ("b", Some("g")), ("c", None)] {
        radio(&fig, name, group, &log).unwrap();
    }
    fig.trigger_tool("a");
    fig.trigger_tool("c");
    fig.trigger_tool("b");
    assert_eq!(
        (
            fig.tool_toggled("a"),
            fig.tool_toggled("b"),
            fig.tool_toggled("c")
        ),
        (Some(false), Some(true), Some(true)),
        "c is in no group"
    );
    fig.trigger_tool("b");
    assert_eq!(fig.toggled_in_group("g"), None, "switched off again");
    let log_now = log.borrow();
    let uses: Vec<(&str, bool)> = log_now.iter().map(|(n, t)| (n.as_str(), *t)).collect();
    assert_eq!(
        uses,
        [
            ("a", true),
            ("c", true),
            ("a", false),
            ("b", true),
            ("b", false)
        ],
        "the tool switched off runs first"
    );
    drop(log_now);
    fig.trigger_tool("a");
    fig.untoggle_group("g");
    assert_eq!(fig.toggled_in_group("g"), None);
    assert_eq!(log.borrow().last(), Some(&("a".to_owned(), false)));
}

#[test]
fn toggles_start_on_and_show_their_cursor() {
    let fig = Figure::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    radio(&fig, "a", Some(NAVIGATION_GROUP), &log).unwrap();
    fig.trigger_tool("a");
    let seen = Rc::clone(&log);
    let hand = ToolSpec {
        radio_group: Some(NAVIGATION_GROUP.into()),
        cursor: Some(CursorShape::Hand),
        default_toggled: true,
        ..spec("hand", ToolKind::Toggle)
    };
    fig.add_tool(hand, move |event| {
        seen.borrow_mut()
            .push((event.name.to_owned(), event.toggled));
    })
    .unwrap();
    assert_eq!(
        *log.borrow(),
        [
            ("a".to_owned(), true),
            ("a".to_owned(), false),
            ("hand".to_owned(), true)
        ],
        "added on, displacing a"
    );
    assert_eq!(
        fig.toggled_in_group(NAVIGATION_GROUP).as_deref(),
        Some("hand")
    );
    assert_eq!(fig.tool_cursor(), Some(CursorShape::Hand));
    fig.trigger_tool("hand");
    assert_eq!(fig.tool_cursor(), None);
    fig.trigger_tool("hand");
    assert!(fig.remove_tool("hand"));
    assert_eq!(fig.tool_cursor(), None, "removed");
    let trigger = ToolSpec {
        default_toggled: true,
        cursor: Some(CursorShape::Wait),
        ..spec("t", ToolKind::Trigger)
    };
    fig.add_tool(trigger, |_| panic!("a trigger does not start on"))
        .unwrap();
    assert_eq!(fig.tool_cursor(), None);
}

#[test]
fn a_callback_using_its_own_group_is_not_run_again() {
    let fig = Figure::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    radio(&fig, "b", Some("g"), &log).unwrap();
    let seen = Rc::clone(&log);
    let spec = ToolSpec {
        radio_group: Some("g".into()),
        ..spec("a", ToolKind::Toggle)
    };
    fig.add_tool(spec, move |event| {
        seen.borrow_mut()
            .push((event.name.to_owned(), event.toggled));
        if event.toggled {
            // Switches a off from inside its own callback.
            event.figure.trigger_tool("b");
        }
    })
    .unwrap();
    fig.trigger_tool("a");
    assert_eq!(
        (fig.tool_toggled("a"), fig.tool_toggled("b")),
        (Some(false), Some(true))
    );
    assert_eq!(
        *log.borrow(),
        [("a".to_owned(), true), ("b".to_owned(), true)]
    );
}
