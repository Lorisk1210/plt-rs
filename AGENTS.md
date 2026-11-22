# pltrs

pltrs is a plotting framework written in Rust. It is a port of
matplotlib 3.11. It has a Rust API and a Python package with the
matplotlib syntax. A matplotlib program runs on pltrs after you change
one import.

## matplotlib

matplotlib is the standard plotting library for Python. A program builds
a figure from axes and artists such as lines, texts and patches. A
backend then writes the figure to a file or shows it in a window.
Libraries such as pandas and seaborn draw their plots through
matplotlib. matplotlib is Python code with C++ parts, and it uses numpy,
FreeType and Agg.

### pltrs in matplotlib context

matplotlib is the specification for pltrs. If pltrs and matplotlib give
different results, pltrs is wrong, unless `DIFFERENCES.md` lists the
difference.

* pltrs (Rust) is the crate `pltrs`. It has the matplotlib model with
  Rust types: option structs, enums and `Result`.
* pltrs (Python) is the package `pltrs`. It has the matplotlib 3.11 API,
  and a native extension does the work. The matplotlib test suite runs
  against it without changes.
* The `matplotlib` alias package loads pltrs under the name
  `matplotlib`. With it, pandas and seaborn draw with pltrs.

pltrs has two looks. The modern look is the default. In the matplotlib
look, the output must match the matplotlib output, and all parity checks
use this look.

All code is in this repository: fonts, image formats, drawing, text
layout, windows and the CPython binding. pltrs has no crates.io
dependencies.

## Vision

The aim is a drop-in replacement for matplotlib that is better for the
user. The user changes one import. The figures are the same or look
better, and they are ready in less time.

Version 0.1.0 is released. A stable release is more important than
identical bytes. If a figure matches the matplotlib figure, the result
is correct, even when the file bytes differ.

## A note from the developer

I like ambitious ideas, simple systems, and software that feels obvious.
Do not preserve complexity just because it already exists. Do not
introduce machinery because it looks architecturally impressive.
Understand the real constraint, then fight for the smallest model that
makes the correct behavior unsurprising.

Channel both "measure twice, cut once" and "yagni". Fight scope creep.
Try to honor my intent in both a minimal and realistic fashion.

When you talk to me, use the /unslop skill and write in precise
language. ASD-STE100 Simplified Technical English is the model.

matplotlib decides the correct behavior. The goldens in `spec/goldens`
are matplotlib output. Do not edit a golden by hand.

Do not add a crates.io dependency, and do not add a dev-dependency.

[`CONTRIBUTING.md`](CONTRIBUTING.md) has the toolchain, the Rust rules
and all check commands. Read [`python/DESIGN.md`](python/DESIGN.md)
before you work on pltrs (Python).

If you build in a private target directory, delete it when you finish.
If you start Xvfb or weston for a test, stop it when you finish.

## Commits and versions

The lead agent integrates and commits. Subagents do not commit and do
not push.

A commit subject names the area and then the change, for example
`axes: keep the limits of a shared axis after a clear`.

pltrs follows Semantic Versioning. `Cargo.toml` is the only source of
the version. The crates, the wheels and the alias package read it.
During 0.x, a release can change the Rust API.

[`RELEASING.md`](RELEASING.md) has the release procedure. A version on
PyPI or crates.io is permanent. Ask me before you run a step that
publishes.

## Design & Taste

Users see three things: the figures in the modern look, the figure
windows, and the documentation. Each must be clear without an
explanation.

It is not enough that a figure, a window or a document works. It must be
exceptional. Critique your result without bias until you can ship it.
To review how the modern look draws a feature, run
`cargo xtask render --look modern PREFIX...`.

If you write text, use the /unslop skill and the documentation style in
`CONTRIBUTING.md`. Write short sentences with plain words, and write
only what the reader needs.

Avoid common AI patterns in text and in design.
