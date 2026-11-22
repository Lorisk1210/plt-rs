# matplotlib's public API in pltrs (Rust)

`cargo xtask rust-coverage [--strict] [--no-doc]` measures how much of
the public API of matplotlib is public Rust API in pltrs (Rust). Each
public callable of matplotlib gets the pltrs (Rust) items that do its
work, or the reason that it does not apply to Rust.

* The callables are those of `cargo xtask diff --coverage`. They are the
  public functions, classes, methods and properties of matplotlib 3.11.
  Class and static methods count as methods. Members of private bases of
  public classes also count. Deprecated callables do
  not count. `api.py` lists them in the reference venv, with
  `spec/diff/api_index.py` and `spec/api-audit/enumerate.py`.
* The ledger, `ledger/<module>.txt`, decides each callable.
* The check requires that every path in the ledger is a public item,
  or a member of one, that rustdoc documents. The command runs
  `cargo doc --no-deps` for the library crates, or with `--no-doc` reads
  an earlier build. It reads the `all.html` of each crate and the pages
  of its items, in the HTML of stable rustdoc.

The command prints the covered share per module, the unmapped callables
by class and the problems of the ledger. It writes the same to
`target/rust-coverage/report.md`. `target/rust-coverage/mapping.tsv`
gives the decision for each callable: the paths or the reason, and the
rule that decided it.

`--strict` fails in these cases:

* A callable is unmapped.
* A ledger line names a callable or an item that no longer exists.
* A ledger line decides nothing.

If a path names nothing, the report lists the items with the same name,
to help you find a renamed or moved item.

## The ledger

The ledger has one file per matplotlib module, named after the module.
Each line is a rule:

```text
Axes.hist -> pltrs::axes::Axes::hist
Axes.get_* -> pltrs::axes::Axes::*
Axes.* -> pltrs::axes::Axes::*
*.set_lw n/a alias of set_linewidth: pltrs keeps one name per property
ArtistInspector n/a Python introspection helper (typed getters replace it)
```

* A pattern is the key of a callable in the module: `Owner.name` for a
  class member, `name` at module level.
* One `*` matches any run of letters, digits and underscores. In the
  paths, `*` stands for the matched text. For example, `Axes.get_xlim`
  becomes `pltrs::axes::Axes::xlim`.
* `-> PATH, PATH...` maps the callable to the pltrs (Rust) items that do
  its work. `n/a REASON` gives the reason that it does not apply to Rust.
* A rule that names the callable exactly wins, at any place in the
  file. It can point at anything that rustdoc documents.
* If not, the first matching wildcard rule wins. For a mapping, every
  path of that rule must name an item of the workspace, not a method of
  a standard trait such as `Clone`. Thus a group rule applies only where
  the item exists.
* `#` starts a comment. `## Class` headings group the rules of a class.

## How the callables map

* Same name on the Rust type of the class. Most classes have one Rust
  type (`Line2D`, `Axes`, `Colormap`, ...). Their group rules map members
  with the same name. Getters drop the `get_` prefix: `get_xlim` is
  `xlim`, as pltrs names getters.
* Different names, mapped by hand. The rules for all other callables
  come from the porting guide (`pltrs::porting`) and the API audit
  (`spec/api-audit`).
* `Artist.set(**kwargs)`, `Artist.update` and `pyplot.setp` map to
  `Line2D::update_props`, `Patch::update_props`, `Text::update_props` and
  `Collection::update_props`. For other artists, they map to
  `ArtistHandle::update_base_props`, the properties that all artists
  share. The other properties have typed setters, which chain.
* Registries (`matplotlib.colormaps`, `color_sequences`, and the
  projection, backend and units registries) map to the functions of the
  module that holds the registry (`pltrs::colors::colormaps::get`,
  ...). Their dunder methods (`__getitem__`, `__iter__`, `__len__`) map
  to the lookups and name lists.
* Not applicable. This is the machinery of Python, and each line gives
  its reason. Examples are property aliases (`set_lw`), and protocol and
  dataclass methods that Rust replaces with traits or plain values
  (`__repr__`, `__reduce__`, `__eq__`, ...). Others are `kwdoc`,
  `ArtistInspector`, file objects, Python GUI toolkits, backend modules
  and internals that are public by accident, such as the streamplot
  integration grid.

## Keep the ledger up to date

* If you rename or move a pltrs (Rust) item, `--strict` names the ledger
  lines that point at it.
* A new matplotlib callable is unmapped until a rule decides it.
* Map new Rust API in the ledger.
