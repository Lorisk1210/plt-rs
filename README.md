# pltrs

pltrs is a plotting framework written in Rust. It is a port of
matplotlib 3.11.

pltrs also has a Python package that uses the matplotlib syntax. You can
use it as a drop-in replacement for matplotlib.

pltrs has no dependencies on crates.io. The code for fonts, image
formats, drawing, text layout and windows is in this repository.

## pltrs (Rust)

The crate `pltrs` has the matplotlib model: figures, axes, artists and a
`pyplot` module. Where matplotlib uses keyword arguments, strings and
exceptions, pltrs (Rust) uses option structs, enums and `Result`.

```rust,no_run
use pltrs::figure::Figure;
use pltrs::numeric::linspace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fig = Figure::new();
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 200);
    ax.plot(&x, x.iter().map(|x| x.sin()))?;
    ax.set_title("A sine");
    ax.set_xlabel("x");
    fig.savefig("sine.png")?;
    pltrs::gui::show(&[fig])?;
    Ok(())
}
```

`savefig` writes PNG, JPEG, TIFF, GIF, WebP, AVIF, SVG, PDF, PostScript
and PGF files. `show` opens a window with tools to pan, zoom and save.
Windows open on Linux (Wayland and X11), macOS and Windows. pltrs can
also show a figure in a web browser.

The [API documentation](https://docs.rs/pltrs) has a tutorial in the
module `pltrs::tutorial`. The directory
[`examples/`](https://github.com/Lorisk1210/plt-rs/blob/main/examples/README.md)
has <!--fact:examples.graphs-->20 graphs, each in Rust and in Python.

## pltrs (Python)

The package `pltrs` has the matplotlib 3.11 API. Its classes and
functions have the same names and signatures as in matplotlib. A native
extension written in Rust does the work.

```python
import pltrs.pyplot as plt

fig, ax = plt.subplots()
ax.plot([1, 2, 3], [4, 1, 3], "o--", label="data")
ax.set_title("pltrs")
ax.legend()
fig.savefig("figure.png")
```

pltrs (Python) runs the test suite of matplotlib 3.11 without changes.
All <!--fact:mpl-suite.all-passing-->10,205 tests that pass with
matplotlib also pass with pltrs. pltrs also works in Jupyter.

In the [benchmarks](https://github.com/Lorisk1210/plt-rs/blob/main/benchmarks/README.md),
pltrs (Python) draws a figure <!--fact:bench.python.warm-->4.3× faster
than matplotlib in a running program. A complete script, with the start
of Python, is <!--fact:bench.python.cold-->1.79× faster. These numbers
are geometric means over <!--fact:bench.compared-->91 figures.

## Looks

pltrs has two looks for figures and windows.

* The modern look is the default. It uses the Inter font, a light grid,
  no top or right spine, left-aligned titles and constrained layout.
* The matplotlib look draws figures as matplotlib draws them. In this
  look, the PNG output has the same pixels as the matplotlib output.

Set an environment variable to select the look:

```sh
PLTRS_LOOK=matplotlib python my_script.py           # the matplotlib look
PLTRS_FIGURE_LOOK=modern-dark python my_script.py   # the modern look on a dark background
```

The variables have the same effect on a Rust program. In Rust code, you
can also call `pltrs::look::set_appearance`.

## Coming from matplotlib

In Python, change `import matplotlib.pyplot as plt` to
`import pltrs.pyplot as plt`. The other lines of your program stay the
same.
[`DIFFERENCES.md`](https://github.com/Lorisk1210/plt-rs/blob/main/DIFFERENCES.md)
lists the known differences from matplotlib. In Rust, the module
`pltrs::porting` of the API documentation shows the pltrs call for each
matplotlib call.

## Installing

```sh
pip install pltrs    # Python
cargo add pltrs      # Rust
```

pltrs (Python) needs Python <!--fact:python.minimum-->3.12 or later.
Wheels are available for Linux, macOS and Windows. On other systems, pip
builds pltrs from source and needs Rust 1.99 or later.

## pandas, seaborn and other libraries

Libraries such as pandas and seaborn import `matplotlib` by name. They
do not import `pltrs`.

The `matplotlib` alias package solves this. It is a second package with
the name `matplotlib`, and it loads pltrs. After you install it, pandas
and seaborn call `matplotlib` as before, but pltrs draws the figures.
The figures have the matplotlib look.

The alias package is not on PyPI, because the name `matplotlib` belongs
to matplotlib there. Download its wheel from the
[GitHub release](https://github.com/Lorisk1210/plt-rs/releases) and
install it together with pltrs:

```sh
pip install pltrs matplotlib-*.whl
pip install pandas seaborn
```

The version of the alias package is <!--fact:alias.version-->`3.11.2+pltrs.0.1.1`.
pip accepts it for a requirement such as `matplotlib>=3.4` and does not
download matplotlib.

You cannot install the alias package and matplotlib in the same
environment. To go back to matplotlib, run
`pip install --force-reinstall matplotlib`.

pltrs runs the test suites of <!--fact:ecosystem.libraries-->75
libraries that use matplotlib, including pandas and seaborn.
Of the <!--fact:ecosystem.reference-passing-->24,559 tests that pass
with matplotlib, <!--fact:ecosystem.pltrs-passing-->24,559 pass with
pltrs.

## Crates

| Crate            | Contents                                                  |
|------------------|-----------------------------------------------------------|
| `pltrs`          | Figures, axes, artists, pyplot, windows and the web view  |
| `pltrs-core`     | Points, transforms, bounding boxes, paths and colors      |
| `pltrs-codec`    | PNG, JPEG, TIFF, GIF, WebP and AVIF, DEFLATE, checksums   |
| `pltrs-font`     | TrueType, OpenType and Type 1 fonts, and the font manager |
| `pltrs-raster`   | The anti-aliased rasterizer, strokes, dashes and clipping |
| `pltrs-text`     | Text layout, mathtext and usetex                          |
| `pltrs-render`   | The PNG, SVG, PDF, PostScript and PGF backends            |
| `pltrs-platform` | Windows and input on Wayland, X11, macOS and Windows      |
| `pltrs-ui`       | The user interface of the figure windows                  |
| `pltrs-python`   | The native extension of pltrs (Python)                    |
| `pltrs-spec`     | The conformance tests against matplotlib                  |
| `xtask`          | Developer commands (`cargo xtask`)                        |

Your program depends only on `pltrs`. The first nine crates are on
crates.io and always have the same version. The last three crates are
not published.

## Building

You need Rust 1.99. The file `rust-toolchain.toml` selects it.

```sh
cargo build --workspace
cargo test --workspace
```

To build and test pltrs (Python) from the source tree:

```sh
cargo xtask python setup       # create the virtual environments
cargo xtask python --release   # build the extension into python/pltrs
cargo xtask python test        # run the Python tests
PYTHONPATH=python .venv-pltrs/bin/python my_script.py
```

These documents have more information:

* [`CONTRIBUTING.md`](https://github.com/Lorisk1210/plt-rs/blob/main/CONTRIBUTING.md)
  has the code rules and all check commands.
* [`python/DESIGN.md`](https://github.com/Lorisk1210/plt-rs/blob/main/python/DESIGN.md)
  explains the design of pltrs (Python).
* [`RELEASING.md`](https://github.com/Lorisk1210/plt-rs/blob/main/RELEASING.md)
  explains how to publish a release.

## License

pltrs has two licenses, and you can use it under either one: the MIT
license (`LICENSE-MIT`) or the Apache License 2.0 (`LICENSE-APACHE`). If
you contribute to pltrs, your contribution has the same two licenses,
unless you state a different license.

pltrs contains code ported from matplotlib and from other projects. It
also contains fonts and data files. These parts keep their licenses.
`THIRD_PARTY.md` lists them, and `LICENSES/` has the license texts.

pltrs is an independent project. It has no connection to the Matplotlib
Development Team or to NumFOCUS, and they do not endorse it.
