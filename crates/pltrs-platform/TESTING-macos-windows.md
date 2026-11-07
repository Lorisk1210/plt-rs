# Testing the macOS and Windows backends

The macOS (`AppKit`) and Windows (Win32) backends of `pltrs-platform`
are compiled and linted on Linux for `aarch64-apple-darwin`,
`x86_64-apple-darwin`, `x86_64-pc-windows-msvc` and
`aarch64-pc-windows-msvc`, and their pure parts (key tables, modifier
tracking, `AltGr`, message decoding, unit conversions, pixel formats)
are unit-tested there. On Linux, the Windows build also runs under Wine
(see [Windows under Wine](#windows-under-wine) below, which says what
that covered). Everything else that talks to the operating system is
tested by hand, on real machines, with this plan.

Run every step on macOS (a Retina laptop, and an external non-Retina
display if there is one) and on Windows 10 or 11 (ideally with two
monitors at different scale settings, e.g. 100 % and 150 %). Keep the
terminal visible: the examples print what they receive.

## 0. Setup

* macOS: the Xcode command-line tools (`xcode-select --install`) for the
  linker, and Rust from rustup.
* Windows: Rust from rustup with the MSVC toolchain and the Visual Studio
  Build Tools (the C++ workload: `link.exe` and the C runtime). The
  backend itself needs no Windows SDK import libraries: `user32`, `gdi32`
  and `kernel32` are linked with `raw-dylib`.

```sh
cargo build -p pltrs-platform --examples
cargo test -p pltrs-platform
cargo clippy -p pltrs-platform --all-targets -- -D warnings
```

Expected: all three pass. `cargo test -p pltrs-platform` runs the
headless and pure tests (the X11 and Wayland live tests skip themselves
without `$DISPLAY`/`$WAYLAND_DISPLAY`); on Windows also the Win32 live
tests (`tests/windows.rs`), which open a few windows for a moment and
use the clipboard.

## 1. The platform example

```sh
cargo run -p pltrs-platform --example window
```

Expected at once:

* A window titled "pltrs-platform demo" shows an animated colour
  gradient (it drifts slowly, about 60 frames a second). On macOS it is
  centered and comes to the front, the application has a Dock icon and a
  menu bar whose first menu (named after the executable, `window`) has
  "Quit window ⌘Q". On Windows it appears where Windows places new
  windows, with the keyboard focus.
* The terminal prints `macos window WindowId(0x…), scale factor 2` on a
  Retina display (1 otherwise), or `windows window WindowId(0x…), scale
  factor 1.5` at 150 % (1 at 100 %, 1.25 at 125 %), then the list of
  keys, then `Focused(true)`.
* The content area is 640 × 400 logical pixels: 1280 × 800 physical
  pixels at scale 2. Check with a screenshot tool: the gradient fills
  the content area exactly, with no black band at the right or bottom.

### 1.1 Window management

| Do | Expect |
| --- | --- |
| Drag the bottom-right corner, release | `Resized(PhysicalSize { … })` lines while dragging; the gradient fills the new size once the button is released (while a border is dragged, the system runs its own loop: newly uncovered areas stay black until release, on both systems) |
| Shrink the window as far as it goes | It stops at 160 × 100 logical pixels (the example's minimum) |
| Minimize (yellow button / `_`), restore from the Dock / task bar | The animation is still running after the restore; no error |
| Cover the window with another one, uncover it | The content comes back at once (repainted from the kept frame) |
| Click another application, then the window | `Focused(false)`, then `Focused(true)` |
| Move the window to a monitor with another scale (macOS: Retina ↔ external; Windows: 100 % ↔ 150 %) | `ScaleFactorChanged(…)` with the new scale, then `Resized(…)` with the physical size for the same logical size; the gradient stays sharp (no blur, no half-size image) |
| Maximize (green button on macOS enters full screen; `□` on Windows) | `Resized(…)`, the gradient fills the screen; restore gives the old size back |

### 1.2 Keys of the example

Press each key with the window focused (no modifier held):

| Key | Expect |
| --- | --- |
| `t` | The title becomes "pltrs-platform demo (1)", then (2), … |
| `r` | The window becomes 800 × 500 logical pixels (`Resized` printed), `r` again: back to 640 × 400 |
| `f` | Full screen (macOS: its own Space, animated), `f` again: back |
| `p` | "animation paused": the gradient freezes; covering and uncovering the window still shows it; `p` again resumes |
| `w` | One second later: "woken up by another thread" (a proxy wake-up from another thread) |
| `0` … `7` | The pointer over the window becomes: arrow, pointing hand, crosshair, move (macOS: open hand; Windows: four arrows), wait (macOS: arrow, as macOS has no application busy cursor; Windows: busy circle), I-beam, left-right arrows, up-down arrows. It reverts to the system's cursors over the title bar and borders, and comes back over the content |
| `c` | "copied text". Paste in a text editor: `copied from pltrs-platform` |
| `v` | Copy some text in another application first: `ClipboardText(Some("…"))` with that text; with an image on the clipboard: `ClipboardText(None)` |
| `i` | "copied the window as a PNG image". macOS: Preview → File → New from Clipboard shows the gradient. Windows: paste into a browser's rich editor, Word or GIMP (the clipboard holds the registered `PNG` format; Paint only reads bitmaps and will not paste it) |
| `Escape` | "Escape pressed: closing", "window closed", the program ends |

### 1.3 Mouse

| Do | Expect |
| --- | --- |
| Move the pointer into the window, around, out | `CursorEntered { position }`, `CursorMoved { position, modifiers }` (positions in physical pixels, (0, 0) at the top-left corner of the content, growing right and down), `CursorLeft` |
| Click the left, middle and right buttons | `MouseInput { button: Left/Middle/Right, state: Pressed, …, click_count: 1 }` then `Released` |
| Double- and triple-click quickly | `click_count: 2`, then `3` (Windows follows the double-click speed set in the mouse settings) |
| Side buttons (back/forward), if the mouse has them | `button: Back` / `Forward` |
| macOS: Control+click, Option+click | `button: Right` (with `CONTROL` in the modifiers), `button: Middle` (with `ALT`), as in matplotlib's macOS backend; both releases report the same button |
| Press inside, drag outside the window, release | `CursorMoved` keeps coming outside (negative or too large positions), then the release |
| Wheel up / down | `MouseWheel { delta: Lines { x: 0.0, y: 1.0 } }` / `y: -1.0` (on macOS with natural scrolling the signs follow the system setting) |
| Tilt the wheel / scroll sideways | `x` positive to the right |
| Touchpad two-finger scroll | macOS: `Pixels { … }` (physical pixels); Windows: fractional `Lines` |
| Hold Shift/Control while moving or clicking | The modifiers appear in the events |

### 1.4 Keyboard

| Do | Expect |
| --- | --- |
| `a`, Shift+`a` | `Character('a')` with `text: Some("a")`, then `Shift` pressed with `modifiers: {}`, then `Character('A')` with `text: Some("A")` and `modifiers: {SHIFT}` |
| Release Shift | `Shift`, `Released`, `modifiers: {SHIFT}` (modifiers are those before the event) |
| Control+`s` (macOS also Command+`s`) | `Character('s')`, `text: None`, `modifiers: {CONTROL}` / `{SUPER}` |
| Arrows, Home, End, Page Up/Down, Delete, Backspace, Tab, Enter (and keypad Enter), F1…F12 | `Left`, `Right`, `Up`, `Down`, `Home`, `End`, `PageUp`, `PageDown`, `Delete`, `Backspace`, `Tab`, `Enter`, `F(1)`… with `text: None` |
| Hold `x` | Repeated presses with `repeat: true`, one release |
| Keypad digits (Num Lock on, on Windows) | `Character('1')`… with text |
| Caps Lock, then `a` | `CapsLock` pressed (macOS: pressed when it turns on, released when it turns off); `Character('A')` |
| Windows, German or French layout: AltGr+`e` (or AltGr+`q` for `@` on German) | `AltGr` pressed with `modifiers: {}` (no fake Control press before it), then `Character('€')` with `text: Some("€")` and `modifiers: {}` |
| Windows, dead keys (German `´` then `e`, French `^` then `e`, or US-International `'` then `e`) | The dead key is `Dead('´')` / `Dead('^')` / `Dead('\'')` with `text: None`; the next key reports `Character('e')` with `text: Some("é")` / `"ê"` |
| Windows: a dead key, then a letter it does not combine with (`´` then `x`) | The letter reports `text: Some("´x")` |
| macOS, US layout: Option+`e`, then `e` | `Dead('´')` with `text: None` (and `modifiers: {ALT}`), then `Character('e')` with `text: Some("é")` |
| macOS, German layout: `´` (the key left of Backspace), then `e`; Option+`u`, then `u` on a US layout | `Dead('´')`, then `text: Some("é")`; `Dead('¨')`, then `text: Some("ü")` |
| macOS: a dead key, then `x`; a dead key, then Command+`c`, then `e` | `text: Some("´x")`; Command+`c` ends the pending accent, so `e` types `e` |
| macOS live test: `PLTRS_MACOS_LIVE=1 cargo test -p pltrs-platform --test macos -- input::dead_keys_compose_with_the_layout` | Passes, or says the layout has no dead acute on Option+`e` or `´` |
| Both: in a figure window's save dialog (Ctrl+S / ⌘S), type a dead key then a letter | The file name field shows the accented letter |
| Windows: Alt alone, then `a` | `Alt` pressed/released; the next `a` is a normal key (no menu mode swallowing it) |
| Windows: Alt+Space | The system menu opens |
| macOS: ⌘Q (or Quit in the menu, or the Dock icon's Quit) | The window closes ("window closed": the example answers `CloseRequested` by closing it), the program ends |
| Windows: Alt+F4 | The window closes ("window closed"), the program ends |

### 1.5 Closing

The close button (red dot / `×`) closes the window ("window closed": the
example answers `CloseRequested` by closing it, without printing it),
and the program ends.

### 1.6 Screenshot

```sh
cargo run -p pltrs-platform --example window -- --screenshot window.png
```

Expected: after a second, "saved window.png" and the program ends.
`window.png` is the gradient at the window's physical size (1280 × 800
at scale 2), the same colours as on screen.

## 2. A UI toolkit window

```sh
cargo run -p pltrs-ui --example canvas_window
```

Expected: a canvas with a drawn figure above a toolbar and a status line.
Hovering the axes shows data coordinates in the status line; clicking
toolbar buttons prints them; hovering a button highlights it; Tab moves
the focus between buttons; Escape or the close button quits.

## 3. Figure windows from Rust

```sh
cargo run -p pltrs --example figure_window
cargo run -p pltrs --example widgets
```

`figure_window` shows three subplots with matplotlib's toolbar. Check:

| Do | Expect |
| --- | --- |
| Look at the text and lines | Sharp on Retina / at 150 % (drawn at the physical resolution) |
| Look at the Dock (macOS), the title bar and the task bar (Windows) | pltrs's contour icon (violet, with viridis bands), sharp, and in the Dock as large as the other icons; on Windows sharp again after moving the window to a monitor of another scale |
| Hover the axes | Coordinates in the toolbar's message area |
| Pan tool (`p`), drag | The view pans; the cursor is the move cursor |
| Zoom tool (`o`), drag a rectangle | The rubber band shows and the view zooms; the cursor is a crosshair |
| Mouse wheel over an axes | Zooms around the pointer |
| `h`, then `left`/`right` (or the mouse's side buttons) | Home view, then back and forward through the views |
| `g`, `G`, `l`, `k` | Major and minor grids, log y and log x |
| `F1` | The list of shortcuts, in a window of its own, drawn at once; the program stays idle (no CPU use) while it is open |
| `f` | Full screen and back |
| `s` (or Ctrl+`s`) | The save dialog; save a PNG and open it |
| Ctrl+`c` on Windows, ⌘C on macOS | The figure is copied as an image (paste as in 1.2 `i`) |
| ⌘W on macOS, Ctrl+`w` on Windows, `q` | The window closes (matplotlib's `keymap.quit`: on macOS the Command key is named `cmd`, as in matplotlib) |
| The figure options button, a text field: type, ⌘A/⌘C/⌘V on macOS (Ctrl on Windows) | Typing and the clipboard work in the fields |
| Resize the window | The figure redraws at the new size on release |

`widgets` shows sliders and a button: dragging the sliders updates the
wave, dragging over it selects a span shown below.

Run `figure_window` once more from a terminal that is not in front
(macOS: start it, then click elsewhere before the window appears): the
window still comes to the front.

## 4. pltrs (Python)

Once pltrs (Python)'s extension builds on these systems (`cargo xtask
python`, then `PYTHONPATH=python`):

```sh
python -c "import pltrs.pyplot as plt; plt.plot([1, 3, 2]); plt.show()"
python -c "import pltrs.pyplot as plt, numpy as np
x = np.linspace(0, 6, 200)
plt.ion(); line, = plt.plot(x, np.sin(x))
for i in range(100):
    line.set_ydata(np.sin(x + i / 10)); plt.pause(0.03)"
```

Expected: the first shows a window and returns when it is closed; the
second animates the sine wave for about three seconds, the window
staying responsive (it can be moved and resized meanwhile).

## 5. Backend selection

| Do | Expect |
| --- | --- |
| `PLTRS_PLATFORM=headless cargo run -p pltrs-platform --example window -- --screenshot w.png` | `headless window …`, no window appears, `w.png` is written |
| macOS with XQuartz running: `PLTRS_PLATFORM=x11 cargo run -p pltrs-platform --example window` | `x11 window …`: an XQuartz window |
| `PLTRS_PLATFORM=wayland` on macOS or Windows | The program fails with "no windowing backend available: Wayland is only supported on Linux" |
| macOS: call `pltrs::pyplot::show()` from a thread other than the main one | No window (AppKit only runs on the main thread): pyplot falls back to the web backend and prints the figure's address |

## 6. Known limitations

* While a border is dragged (live resize), both systems run their own
  event loop, so the application cannot redraw until the button is
  released: newly uncovered areas stay black meanwhile.
* macOS text input reads `NSEvent`'s characters: dead keys and Option
  compositions work as the keyboard layout types them, but input methods
  (Japanese, Chinese) are not supported. Text is not reported while
  Option, Control or Command is held (the crate's rule for shortcuts).
* The Windows clipboard image is offered in the registered `PNG` format
  only.

## Windows under Wine

`tools/wine.sh` installs Wine 10.0 and the MinGW-w64 runtime for the
UCRT without root, from Debian's packages, and builds pltrs for
`x86_64-pc-windows-gnu`, linked by `rust-lld`, running tests and
programs through Wine. Its header lists the commands; in short:

```sh
crates/pltrs-platform/tools/wine.sh install   # once
crates/pltrs-platform/tools/wine.sh python    # once: CPython for Windows, pltrs's requirements
Xvfb :121 -screen 0 1280x800x24 -nolisten tcp &
DISPLAY=:121 crates/pltrs-platform/tools/wine.sh cargo test -p pltrs -p pltrs-platform
DISPLAY=:121 crates/pltrs-platform/tools/wine.sh cargo run -p pltrs --example figure_window
```

`wine.sh x11 xdotool ...` sends input to Wine's windows and `wine.sh x11
xwd -root ...` captures the screen.

**What Wine verified** (2026-11-03; Wine 10.0 on Xvfb, without a window
manager and in Wine's virtual desktop):

* `cargo test` of `pltrs-core`, `pltrs-codec`, `pltrs-font`,
  `pltrs-raster`, `pltrs-text`, `pltrs-render`, `pltrs-platform` and
  `pltrs`, doctests included, and the Win32 live tests: everything
  passes.
* Section 1: the gradient in the right colours at 640 × 400; `t`, `r`,
  `f`, `p`, `w`, `0`–`7` (the cursor is set, its look not checked), `c`
  and `v`, `i` (another Windows program reads the `PNG` format back),
  Escape and Alt+F4; entering, moving, leaving, the five buttons, double
  and triple clicks, both wheels, dragging outside with the pointer
  captured; characters with Shift and Caps Lock, Control+`s`, the named
  keys, keypad Enter, F1–F12, auto-repeat, Alt alone then `a`; 1.6.
* Section 2: the canvas, the toolbar and the coordinates in the status
  line.
* Section 3: `figure_window` draws; hovering, `g`, the zoom rubber band,
  panning, the wheel and `F1`; `widgets`' span selector.
* Section 4 without a window: CPython 3.14.8 for Windows runs pltrs
  (Python)'s smoke tests (`python/tests`) and saves PNG, PDF, SVG, PS,
  EPS, JPEG and TIFF.

**What still needs a real Windows machine:** the MSVC build itself;
DPI scaling (Wine runs at 96 dpi: per-monitor awareness,
`WM_DPICHANGED`, 150 %); other keyboard layouts (AltGr, dead keys) and
input methods; what the shell does (snapping, maximizing from the title
bar, the task bar and Alt+Tab, the minimum size while dragging a border,
live resize); the clipboard across programs and after the program
exits; the cursors' look; and the numbers of Microsoft's UCRT, which
Wine's is not. (The tests comparing with matplotlib's numbers bit for
bit, which matplotlib computed with glibc's `sin` and `cos`, do so with
glibc only; with other C libraries they allow the last digits to
differ.)

**Wine's own limits** (not pltrs's):

* A window wholly inside an older one at the same place stays black
  until it moves (a plain Win32 program does the same). Wine opens every
  window at the screen's corner, so pltrs's dialogs (help, options,
  save) show black there; Windows cascades new windows.
* Wine 10.0's UCRT lacks `crealf`, `cimagf` and C99's complex functions,
  which numpy calls: `wine.sh python` points numpy's math imports at a
  shim holding the accessors, but numpy's complex ufuncs (`np.exp` of a
  complex array) still stop the program.
* `CreateSymbolicLinkW` reports success without making the link.
* `WM_MOUSEHWHEEL` carries client coordinates, where Windows sends screen
  coordinates: horizontal scrolling reports a position off by the window's
  client origin.

With the GNU toolchain (MSYS2, rustup's `-gnu` toolchains), LLVM turns
the sine and cosine of one angle into MinGW-w64's x87 `sincos`, wrong in
the last digits near multiples of π/2 and entirely for large angles;
`wine.sh` links a `sincos` calling the UCRT, as MSVC builds compute.

## Reporting back

For each failing row: the system and version, the display scale, the
keyboard layout for keyboard rows, what happened, and the terminal
output (`RUST_BACKTRACE=1` for panics).
