# Testing the macOS and Windows backends

This plan tests the macOS (`AppKit`) and Windows (Win32) backends of
`pltrs-platform` by hand, on real machines. Linux checks only these
parts:

* The backends compile and pass clippy for `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc`.
* Unit tests cover the pure parts: key tables, modifier tracking,
  `AltGr`, message decoding, unit conversions and pixel formats.
* The Windows build runs under Wine. See
  [Windows under Wine](#windows-under-wine).

Run every step on macOS and on Windows 10 or 11. On macOS, use a Retina
laptop and, if possible, an external non-Retina display. On Windows, use
two monitors at different scales if possible, for example 100 % and
150 %. Keep the terminal visible, because the examples print the events
that they receive.

## 0. Setup

Install these tools:

* macOS: the Xcode command-line tools (`xcode-select --install`) for the
  linker, and Rust from rustup.
* Windows: Rust from rustup with the MSVC toolchain, and the C++
  workload of the Visual Studio Build Tools (`link.exe` and the C
  runtime). The backend links `user32`, `gdi32` and `kernel32` with
  `raw-dylib`, so it needs no import libraries from the Windows SDK.

Then run:

```sh
cargo build -p pltrs-platform --examples
cargo test -p pltrs-platform
cargo clippy -p pltrs-platform --all-targets -- -D warnings
```

Expected: all three pass. The X11 and Wayland live tests skip without
`$DISPLAY` and `$WAYLAND_DISPLAY`. On Windows, the Win32 live tests
(`tests/windows.rs`) also run. They open windows briefly and use the
clipboard.

## 1. The platform example

```sh
cargo run -p pltrs-platform --example window
```

Expected immediately:

* A window "pltrs-platform demo" shows a slowly moving color gradient
  at about 60 frames per second.
* On macOS, the window is centered and in front. The application has a
  Dock icon and a menu bar. The first menu, `window`, has "Quit window
  ⌘Q".
* On Windows, the window appears where Windows places new windows, with
  the keyboard focus.
* The terminal prints `macos window WindowId(0x…), scale factor 2` on
  Retina (1 otherwise), or `windows window WindowId(0x…), scale factor
  1.5` at 150 % (1.25 at 125 %, 1 at 100 %).
* Then it prints the list of keys and `Focused(true)`.
* The content area is 640 × 400 logical pixels, 1280 × 800 physical
  pixels at scale 2. A screenshot shows that the gradient fills it
  exactly, with no black band at the right or bottom.

### 1.1 Window management

1. Drag the bottom-right corner, then release.

   Expected: `Resized(PhysicalSize { … })` lines during the drag. After
   the release, the gradient fills the new size. During the drag, both
   systems run their own loop, so new areas stay black.

2. Make the window as small as possible.

   Expected: it stops at 160 × 100 logical pixels, the minimum of the
   example.

3. Minimize the window (yellow button or `_`). Restore it from the Dock
   or task bar.

   Expected: the animation still runs, with no error.

4. Cover the window with another window, then uncover it.

   Expected: the content comes back immediately, repainted from the kept
   frame.

5. Click another application, then the window.

   Expected: `Focused(false)`, then `Focused(true)`.

6. Move the window to a monitor with another scale: Retina and external
   on macOS, 100 % and 150 % on Windows.

   Expected: `ScaleFactorChanged(…)` with the new scale, then
   `Resized(…)` with the physical size for the same logical size. The
   gradient stays sharp, with no blur and no half-size image.

7. Maximize the window (green button for full screen on macOS, `□` on
   Windows). Then restore it.

   Expected: `Resized(…)`, and the gradient fills the screen. The
   restore gives back the old size.

### 1.2 Keys of the example

Press each key while the window has the focus and no modifier is held.

1. Press `t` several times.

   Expected: the title becomes "pltrs-platform demo (1)", then (2), and
   so on.

2. Press `r` twice.

   Expected: the window becomes 800 × 500 logical pixels, and
   `Resized` appears. The second press gives back 640 × 400.

3. Press `f` twice.

   Expected: full screen, on macOS in a separate Space with an
   animation. The second press goes back.

4. Press `p`, cover and uncover the window, then press `p` again.

   Expected: "animation paused", and the gradient stops. The uncovered
   window still shows it. The second press restarts the animation.

5. Press `w`.

   Expected: after one second, "woken up by another thread" (a proxy
   wake-up).

6. With the pointer over the window, press `0` to `7` in sequence.

   Expected: the pointer becomes these cursors in order:

   * an arrow, a pointing hand and a crosshair
   * a move cursor: an open hand on macOS, four arrows on Windows
   * a wait cursor: an arrow on macOS, which has no busy cursor for
     applications, and a busy circle on Windows
   * an I-beam, left-right arrows and up-down arrows

   Over the title bar and borders, the system cursors return. Over the
   content, the selected cursor comes back.

7. Press `c`, then paste into a text editor.

   Expected: "copied text". The editor shows `copied from pltrs-platform`.

8. Copy some text in another application, then press `v`. Repeat with
   an image.

   Expected: `ClipboardText(Some("…"))` with that text, then
   `ClipboardText(None)` for the image.

9. Press `i`. On macOS, choose File, then New from Clipboard in Preview.
   On Windows, paste into Word, GIMP or a browser rich text editor.

   Expected: "copied the window as a PNG image", and the pasted image
   shows the gradient. The Windows clipboard holds the registered `PNG`
   format. Paint reads only bitmaps and cannot paste it.

10. Press `Escape`.

    Expected: "Escape pressed: closing", "window closed", and the program
    ends.

### 1.3 Mouse

1. Move the pointer into the window, around, and out.

   Expected: `CursorEntered { position }`, `CursorMoved { position,
   modifiers }`, then `CursorLeft`. Positions are in physical pixels,
   with (0, 0) at the top-left corner of the content. They grow to the
   right and down.

2. Click the left, middle and right buttons.

   Expected: `MouseInput { button: Left/Middle/Right, state: Pressed, …,
   click_count: 1 }`, then `Released`.

3. Double-click and triple-click quickly.

   Expected: `click_count: 2`, then `3`. Windows uses the double-click
   speed of the mouse settings.

4. If the mouse has side buttons, click back and forward.

   Expected: `button: Back` and `button: Forward`.

5. On macOS, Control+click and Option+click.

   Expected: `button: Right` with `CONTROL`, and `button: Middle` with
   `ALT`, as in the matplotlib macOS backend. Each release reports the
   button of its press.

6. Press inside the window, drag outside, then release.

   Expected: `CursorMoved` continues outside, with negative or too large
   positions. Then the release.

7. Turn the wheel up, then down.

   Expected: `MouseWheel { delta: Lines { x: 0.0, y: 1.0 } }`, then
   `y: -1.0`. On macOS with natural scrolling, the signs follow the
   system setting.

8. Tilt the wheel or scroll sideways.

   Expected: `x` is positive to the right.

9. Scroll with two fingers on a touchpad.

   Expected: `Pixels { … }` in physical pixels on macOS, fractional
   `Lines` on Windows.

10. Hold Shift or Control while you move or click.

    Expected: the modifiers appear in the events.

### 1.4 Keyboard

1. Press `a`, then Shift+`a`.

   Expected: `Character('a')` with `text: Some("a")`. Then `Shift`
   pressed with `modifiers: {}`. Then `Character('A')` with
   `text: Some("A")` and `modifiers: {SHIFT}`.

2. Release Shift.

   Expected: `Shift`, `Released`, `modifiers: {SHIFT}`. The modifiers
   are those before the event.

3. Press Control+`s`, and on macOS also Command+`s`.

   Expected: `Character('s')`, `text: None`, `modifiers: {CONTROL}` or
   `{SUPER}`.

4. Press the arrows, Home, End, Page Up, Page Down, Delete, Backspace,
   Tab, Enter, keypad Enter and F1 to F12.

   Expected: `Left`, `Right`, `Up`, `Down`, `Home`, `End`, `PageUp`,
   `PageDown`, `Delete`, `Backspace`, `Tab`, `Enter`, `F(1)`… with
   `text: None`.

5. Hold `x`.

   Expected: repeated presses with `repeat: true`, then one release.

6. Press the keypad digits, on Windows with Num Lock on.

   Expected: `Character('1')`… with text.

7. Press Caps Lock, then `a`.

   Expected: `CapsLock` pressed, then `Character('A')`. On macOS,
   `CapsLock` is pressed when it turns on, released when it turns off.

8. On Windows with a German or French layout, press AltGr+`e`. AltGr+`q`
   for `@` on German also works.

   Expected: `AltGr` pressed with `modifiers: {}`, with no fake Control
   press before it. Then `Character('€')` with `text: Some("€")` and
   `modifiers: {}`.

9. On Windows, press a dead key, then `e`: German `´`, French `^` or
   US-International `'`.

   Expected: `Dead('´')`, `Dead('^')` or `Dead('\'')` with `text: None`.
   Then `Character('e')` with `text: Some("é")` or `"ê"`.

10. On Windows, press a dead key, then a letter it does not combine
    with, such as `´` then `x`.

    Expected: the letter reports `text: Some("´x")`.

11. On macOS with a US layout, press Option+`e`, then `e`.

    Expected: `Dead('´')` with `text: None` and `modifiers: {ALT}`. Then
    `Character('e')` with `text: Some("é")`.

12. On macOS, press `´` then `e` on a German layout. Press Option+`u`
    then `u` on a US layout.

    Expected: `Dead('´')`, then `text: Some("é")`. `Dead('¨')`, then
    `text: Some("ü")`.

13. On macOS, press a dead key, then `x`. Then press a dead key,
    Command+`c` and `e`.

    Expected: `text: Some("´x")`. Command+`c` cancels the pending accent,
    so `e` types `e`.

14. On macOS, run the live test for dead keys:

    ```sh
    PLTRS_MACOS_LIVE=1 cargo test -p pltrs-platform --test macos -- input::dead_keys_compose_with_the_layout
    ```

    Expected: it passes, or says that the layout has no dead acute on
    Option+`e` or `´`.

15. On both systems, open the save dialog of a figure window (Ctrl+S or
    ⌘S). Type a dead key, then a letter.

    Expected: the file name field shows the accented letter.

16. On Windows, press Alt alone, then `a`.

    Expected: `Alt` pressed and released. The next `a` is a normal key,
    and no menu mode takes it.

17. On Windows, press Alt+Space.

    Expected: the system menu opens.

18. On macOS, press ⌘Q, or choose Quit in the menu or the Dock icon.

    Expected: "window closed", and the program ends. The example closes
    the window on `CloseRequested`.

19. On Windows, press Alt+F4.

    Expected: "window closed", and the program ends.

### 1.5 Closing

1. Click the close button (red dot or `×`).

   Expected: "window closed", and the program ends. The example closes
   the window on `CloseRequested` and does not print the event.

### 1.6 Screenshot

1. Run:

   ```sh
   cargo run -p pltrs-platform --example window -- --screenshot window.png
   ```

   Expected: after one second, "saved window.png", and the program ends.
   `window.png` shows the gradient at the physical size of the window
   (1280 × 800 at scale 2), in the colors of the screen.

## 2. A UI toolkit window

```sh
cargo run -p pltrs-ui --example canvas_window
```

Expected: a canvas with a figure, above a toolbar and a status line.

1. Move the pointer over the axes, then over a toolbar button. Click the
   button.

   Expected: the status line shows the data coordinates. The button is
   highlighted, and the click prints it.

2. Press Tab several times, then Escape. Alternatively, click the close
   button.

   Expected: the focus moves between the buttons, and Escape ends the
   program.

## 3. Figure windows from Rust

```sh
cargo run -p pltrs --example figure_window
```

Expected: three subplots with the matplotlib toolbar.

1. Look at the text and the lines.

   Expected: they are sharp on Retina and at 150 %, because pltrs draws
   at the physical resolution.

2. Look at the Dock on macOS, or the title bar and task bar on Windows.

   Expected: the contour icon of pltrs (violet, with viridis bands) is
   sharp. In the Dock, it is as large as the other icons. On Windows, it
   is sharp again after a move to a monitor with another scale.

3. Move the pointer over the axes.

   Expected: the coordinates appear in the message area of the toolbar.

4. Select the pan tool (`p`), then drag.

   Expected: the view pans, and the pointer is the move cursor.

5. Select the zoom tool (`o`), then drag a rectangle.

   Expected: the rubber band appears and the view zooms. The pointer is
   a crosshair.

6. Turn the mouse wheel over an axes.

   Expected: the view zooms around the pointer.

7. Press `h`, then `left` and `right` or the side buttons of the mouse.

   Expected: the home view, then back and forward through the views.

8. Press `g`, `G`, `l` and `k`.

   Expected: the major grid, the minor grid, log y and log x.

9. Press `F1`.

   Expected: the list of shortcuts opens immediately in a separate
   window. The program uses no CPU while it is open.

10. Press `f` twice.

    Expected: full screen, then back.

11. Press `s` or Ctrl+`s`, save a PNG and open it.

    Expected: the save dialog opens, and the PNG shows the figure.

12. Press Ctrl+`c` on Windows or ⌘C on macOS. Paste as in section 1.2,
    step 9.

    Expected: the figure is pasted as an image.

13. Press ⌘W on macOS or Ctrl+`w` on Windows. Start again and press `q`.

    Expected: the window closes each time. This is `keymap.quit` of
    matplotlib. On macOS, the Command key is `cmd`, as in matplotlib.

14. Click the figure options button. In a text field, type, and use ⌘A,
    ⌘C and ⌘V (Ctrl on Windows).

    Expected: typing and the clipboard work in the fields.

15. Resize the window.

    Expected: after the release, the figure draws at the new size.

16. Run:

    ```sh
    cargo run -p pltrs --example widgets
    ```

    Expected: sliders and a button. A slider drag changes the wave. A
    drag over the wave selects a span, which appears below.

17. On macOS, start `figure_window` from the terminal, then click
    another application before the window appears.

    Expected: the window still comes to the front.

## 4. pltrs (Python)

If the extension of pltrs (Python) builds on the system, run
`cargo xtask python` and set `PYTHONPATH=python`.

1. Run:

   ```sh
   python -c "import pltrs.pyplot as plt; plt.plot([1, 3, 2]); plt.show()"
   ```

   Expected: a window opens. The command returns when you close it.

2. Run:

   ```sh
   python -c "import pltrs.pyplot as plt, numpy as np
   x = np.linspace(0, 6, 200)
   plt.ion(); line, = plt.plot(x, np.sin(x))
   for i in range(100):
       line.set_ydata(np.sin(x + i / 10)); plt.pause(0.03)"
   ```

   Expected: the sine wave moves for about three seconds. You can move
   and resize the window during the animation.

## 5. Backend selection

1. Run:

   ```sh
   PLTRS_PLATFORM=headless cargo run -p pltrs-platform --example window -- --screenshot w.png
   ```

   Expected: `headless window …`, no window, and the program writes
   `w.png`.

2. On macOS with XQuartz running, run:

   ```sh
   PLTRS_PLATFORM=x11 cargo run -p pltrs-platform --example window
   ```

   Expected: `x11 window …`, and an XQuartz window opens.

3. On macOS or Windows, run the example with `PLTRS_PLATFORM=wayland`.

   Expected: it fails with "no windowing backend available: Wayland is
   only supported on Linux".

4. On macOS, call `pltrs::pyplot::show()` from a thread that is not the
   main thread.

   Expected: no window, because AppKit runs only on the main thread.
   pyplot uses the web backend and prints the address of the figure.

## 6. Known limitations

* During a border drag (live resize), both systems run their own event
  loop. The application cannot draw until the release, so new areas
  stay black.
* macOS text input reads the characters of `NSEvent`. Dead keys and
  Option combinations work as the layout types them. Input methods,
  such as Japanese and Chinese, do not work.
* macOS reports no text while Option, Control or Command is held. This
  is the rule of the crate for shortcuts.
* The Windows clipboard gives images only in the registered `PNG`
  format.

## Windows under Wine

`tools/wine.sh` installs Wine 10.0 and the MinGW-w64 runtime for the
UCRT from Debian packages, without root. It builds pltrs for
`x86_64-pc-windows-gnu` with `rust-lld` and runs tests and programs
through Wine. Its header lists all commands. The main ones are:

```sh
crates/pltrs-platform/tools/wine.sh install   # once
crates/pltrs-platform/tools/wine.sh python    # once: CPython for Windows, pltrs's requirements
Xvfb :121 -screen 0 1280x800x24 -nolisten tcp &
DISPLAY=:121 crates/pltrs-platform/tools/wine.sh cargo test -p pltrs -p pltrs-platform
DISPLAY=:121 crates/pltrs-platform/tools/wine.sh cargo run -p pltrs --example figure_window
```

`wine.sh x11 xdotool ...` sends input to the Wine windows.
`wine.sh x11 xwd -root ...` captures the screen.

### What Wine verified

These checks pass with Wine 10.0 on Xvfb, without a window manager and
in the virtual desktop of Wine:

* `cargo test` of `pltrs-core`, `pltrs-codec`, `pltrs-font`,
  `pltrs-raster`, `pltrs-text`, `pltrs-render`, `pltrs-platform` and
  `pltrs`, with doctests and the Win32 live tests.
* Section 1: the gradient in the correct colors at 640 × 400, and the
  keys `t`, `r`, `f`, `p`, `w`, `c`, `v`, `i`, Escape and Alt+F4.
  Another Windows program reads the `PNG` of `i`. Keys `0` to `7` set
  the cursor, but nobody checked its look.
* Section 1.3: enter, move, leave, the five buttons, double and triple
  clicks, both wheels, and a drag outside with the pointer captured.
* Section 1.4: characters with Shift and Caps Lock, Control+`s`, the
  named keys, keypad Enter, F1 to F12, auto-repeat, and Alt alone then
  `a`.
* Section 1.6.
* Section 2: the canvas, the toolbar and the coordinates in the status
  line.
* Section 3: `figure_window` draws. Hover, `g`, the zoom rubber band,
  pan, the wheel, `F1` and the span selector of `widgets` work.
* Section 4 without a window: CPython 3.14.8 for Windows runs the smoke
  tests of pltrs (Python) (`python/tests`). It saves PNG, PDF, SVG, PS,
  EPS, JPEG and TIFF.

### What needs a real Windows machine

* The MSVC build.
* DPI scaling: per-monitor awareness, `WM_DPICHANGED` and 150 %. Wine
  runs at 96 dpi.
* Other keyboard layouts (AltGr, dead keys) and input methods.
* The shell: snapping, maximize from the title bar, the task bar,
  Alt+Tab, the minimum size during a border drag, and live resize.
* The clipboard between programs and after the program ends.
* The look of the cursors.
* The numbers of the Microsoft UCRT, which the UCRT of Wine does not
  reproduce.

matplotlib computed the reference numbers with the `sin` and `cos` of
glibc. The tests compare with them bit for bit only with glibc. With
other C libraries, the tests allow the last digits to differ.

### Limits of Wine

These limits come from Wine, not from pltrs:

* A window inside an older window at the same place stays black until
  it moves. A plain Win32 program does the same. Wine opens every window
  at the screen corner, so the help, options and save dialogs of pltrs
  are black. Windows cascades new windows.
* The UCRT of Wine 10.0 lacks `crealf`, `cimagf` and the C99 complex
  functions that numpy calls. `wine.sh python` points the math imports
  of numpy to a shim with the accessors. The complex ufuncs of numpy
  still stop the program, for example `np.exp` of a complex array.
* `CreateSymbolicLinkW` reports success but makes no link.
* `WM_MOUSEHWHEEL` gives client coordinates, where Windows gives screen
  coordinates. Horizontal scrolling reports a position off by the client
  origin of the window.

With the GNU toolchain (MSYS2, the `-gnu` toolchains of rustup), LLVM
replaces sine and cosine of one angle with the x87 `sincos` of
MinGW-w64. It is wrong in the last digits near multiples of π/2, and
wrong for large angles. `wine.sh` links a `sincos` that calls the UCRT,
as MSVC builds do.

## Reporting back

For each failed step, report the system and its version, the display
scale, and the keyboard layout for keyboard steps. Also report what
happened and the terminal output. For panics, set `RUST_BACKTRACE=1`.
