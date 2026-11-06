# matplotlib's public API in pltrs (Rust)

`cargo xtask rust-coverage [--strict] [--no-doc]` measures how much of
matplotlib's public API pltrs (Rust) offers as public Rust API: for each
of matplotlib's public callables, the pltrs (Rust) items that do what it
does, or why it does not apply to Rust.

* **The callables** are those `cargo xtask diff --coverage` measures the
  differential tester's programs on: matplotlib 3.11's public functions,
  classes, methods, properties, class and static methods, and the members
  of the private bases of public classes (`api.py` lists them in the
  reference venv, through `spec/diff/api_index.py` and
  `spec/api-audit/enumerate.py`). Deprecated ones do not count.
* **The ledger**, `ledger/<module>.txt`, decides each callable (the
  format is below).
* **The check**: every path the ledger names must be a public item, or a
  member of one, that rustdoc documents. The command builds the
  documentation of the library crates (`cargo doc --no-deps`; `--no-doc`
  reads what an earlier build left) and reads each crate's `all.html` and
  the pages of its items: stable rustdoc's HTML, no nightly JSON.

It prints the share covered per module, the unmapped callables by class
and the ledger's problems, and writes the same to
`target/rust-coverage/report.md`, with every callable's decision (the
paths, or the reason, and the rule that decided it) in
`target/rust-coverage/mapping.tsv`. `--strict` fails on unmapped
callables, and on ledger lines that name a callable or an item that is
gone, or decide nothing. A path that names nothing is reported with the
items of the same name, which is where a renamed or moved item went.

## The ledger

One file per matplotlib module, named after it. Each line is a rule:

```text
Axes.hist -> pltrs::axes::Axes::hist
Axes.get_* -> pltrs::axes::Axes::*
Axes.* -> pltrs::axes::Axes::*
*.set_lw n/a alias of set_linewidth: pltrs keeps one name per property
ArtistInspector n/a Python introspection helper (typed getters replace it)
```

* A pattern is a callable's key within the module: `Owner.name` for a
  class' members, `name` at module level. One `*` stands for any run of
  letters, digits and underscores; in the paths it stands for what it
  matched (`Axes.get_xlim` becomes `pltrs::axes::Axes::xlim`).
* `-> PATH, PATH...` maps the callable to the pltrs (Rust) items doing
  it, `n/a REASON` says why it does not apply to Rust.
* A callable takes the rule naming it exactly, wherever it is in the
  file. Otherwise it takes the first wildcard rule that matches it and,
  for a mapping, whose every path names an item of the workspace's own
  (not a method of a standard trait such as `Clone`): a group rule never
  claims a callable by accident, it only applies where the item exists.
  A rule naming a callable exactly may point at anything rustdoc
  documents.
* `#` starts a comment; `## Class` headings group the rules of a class.

## How the callables map

* **Same name on the class' Rust type**: most classes have one Rust type
  (`Line2D`, `Axes`, `Colormap`, ...), so their group rules map members of
  the same name, getters without their `get_` prefix (`get_xlim` is
  `xlim`, pltrs' convention for getters).
* **Different names, mapped by hand**: everything else, from the porting
  guide (`pltrs::porting`) and the API audit (`spec/api-audit`), whose
  verdicts seeded the ledger. Every path was then checked against rustdoc;
  claims that named nothing were mapped again by hand or reported as gaps.
* **`Artist.set(**kwargs)`**, `Artist.update` and `pyplot.setp`: the
  property sets of lines, patches, texts and collections
  (`Line2D::update_props`, `Patch::update_props`, `Text::update_props`,
  `Collection::update_props`), and for every other artist
  `ArtistHandle::update_base_props`, the properties all artists share; an
  artist's own properties have typed setters, which chain.
* **Registries** (`matplotlib.colormaps`, `color_sequences`, the
  projection, backend and units registries) map to the functions of the
  module that holds the registry (`pltrs::colors::colormaps::get`, ...);
  their dunder methods (`__getitem__`, `__iter__`, `__len__`) to the
  lookups and name lists.
* **Not applicable** are Python's own machinery: property aliases
  (`set_lw`), the protocol and dataclass methods Rust replaces with traits
  or plain values (`__repr__`, `__reduce__`, `__eq__`, ...), docstring and
  introspection tools (`kwdoc`, `ArtistInspector`), file objects, Python
  GUI toolkits and backend modules, and matplotlib internals that are
  public by accident (streamplot's integration grid). Each line gives its
  reason.

## Keeping it up to date

When a pltrs (Rust) item is renamed or moved, `--strict` names the ledger
lines that point at it. When matplotlib gains a callable, it shows as
unmapped until a rule decides it. Map new Rust API here as it is added.
