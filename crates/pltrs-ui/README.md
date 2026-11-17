# pltrs-ui

A small retained-mode UI engine, used for pltrs' figure windows:
widgets, layout, painting and input.

* a widget tree: rows and columns, forms, pages and tabs, scrolling
  views, buttons, labels, text fields, lists, sliders and more
* full-colour vector icons with gradients and drop shadows
* layout, painting with `pltrs-raster` and `pltrs-text`, and input
  routing: hover, pointer capture, popups, tooltips
* every control usable from the keyboard, following Qt and GTK

## One of pltrs' crates

pltrs is matplotlib rebuilt from scratch in Rust, with no dependencies
outside its own crates. Its crates are released together, at one
version, and each depends on the exact version of the others: what they
offer each other is not an interface of its own. Most programs want the
crate `pltrs`, matplotlib's model with every output format and
interactive windows; this crate is for those who need its widgets alone.

## Licence

MIT OR Apache-2.0, at your option (`LICENSE-MIT`, `LICENSE-APACHE`).
