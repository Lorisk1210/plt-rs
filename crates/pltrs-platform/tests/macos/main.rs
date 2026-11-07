//! The public API against `AppKit`, on macOS with a window server. The
//! tests open windows on the user's screen, go full screen, set the Dock
//! icon and replace the clipboard's contents, so they run only when asked
//! for with `PLTRS_MACOS_LIVE=1` (a session logged in at the Mac always
//! has a window server, even over ssh). On other systems, without a
//! display, or without the variable, there is nothing to test:
//!
//! ```text
//! PLTRS_MACOS_LIVE=1 cargo test -p pltrs-platform --test macos [-- FILTER...]
//! ```
//!
//! `AppKit` only runs on the main thread, which libtest keeps for its
//! runner, so these tests bring their own (`harness = false`): it runs
//! the tests whose names contain a filter (every test without one) one
//! at a time on the main thread, and reports them as libtest does.
//!
//! The windows open on the screen for a moment. Input is posted to the
//! application's event queue, as the window server delivers it: posting
//! it to the window server instead (`CGEventPost`) needs the
//! Accessibility permission. What views show is read back by drawing
//! them into a bitmap, as reading the screen needs the Screen Recording
//! permission. The pointer and the keyboard stay the user's, so each
//! test looks for the events it caused among those the user may cause.

#![expect(clippy::print_stdout, reason = "the runner reports as libtest does")]

#[cfg(target_os = "macos")]
#[allow(
    unsafe_code,
    reason = "the tests' own FFI to AppKit and Core Graphics, behind safe functions"
)]
mod appkit;
#[cfg(target_os = "macos")]
#[path = "../common/mod.rs"]
mod common;
#[cfg(target_os = "macos")]
mod frames;
#[cfg(target_os = "macos")]
mod input;
#[cfg(target_os = "macos")]
mod services;
#[cfg(target_os = "macos")]
mod windows;

use std::io::Write;
use std::panic;
use std::process::ExitCode;
use std::time::Instant;

/// The variable that lets the tests use the screen and the clipboard.
const LIVE_VARIABLE: &str = "PLTRS_MACOS_LIVE";

/// A live test: its name and its body.
type Test = (&'static str, fn());

/// Every live test, in the order they run.
#[cfg(target_os = "macos")]
const TESTS: &[Test] = &[
    (
        "windows::open_at_the_screens_scale",
        windows::open_at_the_screens_scale,
    ),
    ("windows::titles_show_as_set", windows::titles_show_as_set),
    (
        "windows::sizes_and_minimum_sizes",
        windows::sizes_and_minimum_sizes,
    ),
    (
        "windows::resizing_reports_the_new_size",
        windows::resizing_reports_the_new_size,
    ),
    ("windows::new_windows_cascade", windows::new_windows_cascade),
    (
        "windows::closing_removes_the_window",
        windows::closing_removes_the_window,
    ),
    (
        "windows::icons_go_to_the_dock",
        windows::icons_go_to_the_dock,
    ),
    (
        "windows::rendered_icons_go_to_the_dock_in_every_size",
        windows::rendered_icons_go_to_the_dock_in_every_size,
    ),
    (
        "windows::full_screen_and_back",
        windows::full_screen_and_back,
    ),
    (
        "frames::presented_frames_read_back_exactly",
        frames::presented_frames_read_back_exactly,
    ),
    (
        "frames::views_draw_frames_pixel_for_pixel",
        frames::views_draw_frames_pixel_for_pixel,
    ),
    (
        "frames::views_draw_frames_after_a_resize",
        frames::views_draw_frames_after_a_resize,
    ),
    (
        "input::buttons_press_and_release_where_posted",
        input::buttons_press_and_release_where_posted,
    ),
    (
        "input::control_and_option_clicks_emulate_buttons",
        input::control_and_option_clicks_emulate_buttons,
    ),
    (
        "input::drags_move_the_pointer",
        input::drags_move_the_pointer,
    ),
    (
        "input::the_pointer_enters_and_leaves",
        input::the_pointer_enters_and_leaves,
    ),
    (
        "input::wheels_scroll_lines_and_touchpads_pixels",
        input::wheels_scroll_lines_and_touchpads_pixels,
    ),
    (
        "input::keys_type_text_and_name_keys",
        input::keys_type_text_and_name_keys,
    ),
    (
        "input::dead_keys_compose_with_the_layout",
        input::dead_keys_compose_with_the_layout,
    ),
    (
        "input::modifier_keys_go_down_and_up",
        input::modifier_keys_go_down_and_up,
    ),
    (
        "input::command_w_reaches_the_window",
        input::command_w_reaches_the_window,
    ),
    (
        "input::command_q_asks_every_window_to_close",
        input::command_q_asks_every_window_to_close,
    ),
    (
        "services::timers_and_wake_ups_drive_a_blocking_run",
        services::timers_and_wake_ups_drive_a_blocking_run,
    ),
    (
        "services::text_goes_to_the_clipboard_and_comes_back",
        services::text_goes_to_the_clipboard_and_comes_back,
    ),
    (
        "services::images_go_to_the_clipboard_as_png",
        services::images_go_to_the_clipboard_as_png,
    ),
    (
        "services::cursors_show_over_the_window",
        services::cursors_show_over_the_window,
    ),
];

/// There are no live tests away from macOS.
#[cfg(not(target_os = "macos"))]
const TESTS: &[Test] = &[];

/// A new event loop on `AppKit`.
#[cfg(target_os = "macos")]
fn appkit_loop() -> pltrs_platform::EventLoop {
    use pltrs_platform::{BackendKind, EventLoop};
    let event_loop =
        EventLoop::with_backend(BackendKind::MacOs).expect("AppKit runs on the main thread");
    assert_eq!(event_loop.backend_kind(), BackendKind::MacOs);
    event_loop
}

/// Handles the events of `event_loop` for `time`.
#[cfg(target_os = "macos")]
fn settle(event_loop: &mut pltrs_platform::EventLoop, time: std::time::Duration) {
    let until = Instant::now() + time;
    event_loop
        .run_until(until, |_, _| {})
        .expect("pumping events succeeds");
}

/// Opens a window of `size` and waits for its opening animation to end:
/// until then, `AppKit` places events by where the animation shows the
/// window.
#[cfg(target_os = "macos")]
fn open_still(
    event_loop: &mut pltrs_platform::EventLoop,
    size: pltrs_platform::PhysicalSize,
) -> pltrs_platform::WindowId {
    let window = common::open(event_loop, size);
    settle(event_loop, std::time::Duration::from_millis(500));
    window
}

/// Whether windows can be opened: a window server with displays.
fn has_window_server() -> bool {
    #[cfg(target_os = "macos")]
    return appkit::has_window_server();
    #[cfg(not(target_os = "macos"))]
    false
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| !arg.starts_with('-'))
        .collect();
    let selected: Vec<&Test> = TESTS
        .iter()
        .filter(|(name, _)| filters.is_empty() || filters.iter().any(|f| name.contains(f)))
        .collect();
    if args.iter().any(|arg| arg == "--list") {
        for (name, _) in &selected {
            println!("{name}: test");
        }
        return ExitCode::SUCCESS;
    }
    if std::env::var_os(LIVE_VARIABLE).is_none() {
        println!(
            "\nrunning 0 tests (they use the screen and the clipboard: set {LIVE_VARIABLE}=1)\n"
        );
        return ExitCode::SUCCESS;
    }
    if !has_window_server() {
        println!("\nrunning 0 tests (no macOS window server to open windows on)\n");
        return ExitCode::SUCCESS;
    }
    run(&selected)
}

/// Runs `tests` one after the other; fails if one of them does.
fn run(tests: &[&Test]) -> ExitCode {
    let started = Instant::now();
    println!("\nrunning {} tests", tests.len());
    let mut failed = Vec::new();
    for &&(name, test) in tests {
        print!("test {name} ... ");
        // The panic message goes to stderr, after the name.
        let _ = std::io::stdout().flush();
        if panic::catch_unwind(test).is_ok() {
            println!("ok");
        } else {
            println!("FAILED");
            failed.push(name);
        }
    }
    if !failed.is_empty() {
        println!("\nfailures:");
        for name in &failed {
            println!("    {name}");
        }
    }
    let verdict = if failed.is_empty() { "ok" } else { "FAILED" };
    println!(
        "\ntest result: {verdict}. {} passed; {} failed; finished in {:.2}s\n",
        tests.len() - failed.len(),
        failed.len(),
        started.elapsed().as_secs_f64()
    );
    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
