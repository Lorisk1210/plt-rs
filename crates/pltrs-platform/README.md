# pltrs-platform

Native windowing and input for pltrs on Wayland, X11, macOS and Windows,
without any windowing library or binding crate.

* an event loop that opens windows, shows pixel buffers in them and
  turns input into platform-neutral events
* the X11 and Wayland protocols spoken directly, XKB keymaps and Compose
  files included
* AppKit through the Objective-C runtime on macOS, and Win32 on Windows
* a headless loop for tests, with injected input and recorded frames

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its windows alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`). The
code and data it takes from other projects keep their own licences:
`THIRD_PARTY.md` lists them, with their texts in `LICENSES/`.
