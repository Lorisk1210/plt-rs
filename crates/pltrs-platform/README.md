# pltrs-platform

`pltrs-platform` opens windows and reads input on Wayland, X11, macOS and
Windows. It uses no window library.

Contents:

* An event loop that opens windows, shows pixel buffers and reports input
  as events that are the same on all systems
* The X11 and Wayland protocols, with XKB keymaps and Compose files
* AppKit on macOS and Win32 on Windows
* A headless event loop for tests, with simulated input and recorded
  frames

## Use

Most programs need the crate [`pltrs`](https://crates.io/crates/pltrs).
It has figures, all output formats and windows. Use `pltrs-platform` directly
only if you need the windows alone.

All pltrs crates have the same version. Each crate requires the same
version of the other pltrs crates.

## License

You can use this crate under the MIT license (`LICENSE-MIT`) or the Apache
License 2.0 (`LICENSE-APACHE`).

The code and data from other projects keep their licenses. `THIRD_PARTY.md`
lists them, and `LICENSES/` has the license texts.
