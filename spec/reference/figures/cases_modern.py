"""plt-rs' modern look (``plt::look``), drawn by matplotlib: the style
sheets ``crates/plt/src/style/sheets/plt-modern.mplstyle`` and
``plt-modern-dark.mplstyle`` applied to typical plots, with Inter (which
plt-rs bundles) added to matplotlib's font manager.

The sheets' ``plt.*`` parameters have no matplotlib counterpart and are
left out: the cases do by hand what they ask for (an image turns the grid
of its axes off, error bars of bars take ``plt.ink``, table cells
``plt.table.edgecolor``). The data use exactly rounded arithmetic (no
``exp`` or ``pow``), so that numpy and Rust agree to the last bit.
"""

import pathlib

import numpy as np
from matplotlib import font_manager

from spec_support import case, rng

REPO = pathlib.Path(__file__).resolve().parents[3]

for _font in sorted((REPO / "crates" / "plt-font" / "fonts").glob("Inter-*.ttf")):
    font_manager.fontManager.addfont(str(_font))


def _sheet_entries(name):
    """Every parameter of a plt-rs style sheet."""
    path = REPO / "crates" / "plt" / "src" / "style" / "sheets" / f"{name}.mplstyle"
    params = {}
    for line in path.read_text().splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            key, value = (part.strip() for part in line.split(":", 1))
            params[key] = value
    return params


def _sheet(name):
    """The parameters of a plt-rs style sheet, without its ``plt.*`` ones."""
    return {k: v for k, v in _sheet_entries(name).items() if not k.startswith("plt.")}


MODERN = _sheet("plt-modern")
DARK = _sheet("plt-modern-dark")
#: The colours the sheets' `plt.ink` (dark) and `plt.table.edgecolor`
#: (light) ask for.
INK_DARK = "#" + _sheet_entries("plt-modern-dark")["plt.ink"]
HAIRLINE = "#" + _sheet_entries("plt-modern")["plt.table.edgecolor"]
SIZE = (7.2, 4.5)
X = np.linspace(0, 10, 200)


def _damped(ax):
    for i in range(4):
        ax.plot(X, np.sin(X + 0.7 * i) / (1 + X / 5), label=f"phase {i}")
    ax.set_title("Damped oscillations")
    ax.set_xlabel("time (s)")
    ax.set_ylabel("amplitude")
    ax.legend()


@case("modern/lines", figsize=SIZE, rc=MODERN)
def _(fig):
    _damped(fig.subplots())


@case("modern/dark_lines", figsize=SIZE, rc=DARK)
def _(fig):
    _damped(fig.subplots())


@case("modern/bars", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    x = np.arange(4)
    years = {"2023": [18, 24, 31, 12], "2024": [21, 22, 35, 17], "2025": [26, 25, 33, 23]}
    for i, (year, values) in enumerate(years.items()):
        bars = ax.bar(x + (i - 1) * 0.25, values, 0.25, label=year)
        ax.bar_label(bars, padding=2)
    ax.set_xticks(x, ["north", "south", "east", "west"])
    ax.set_ylabel("units (thousands)")
    ax.set_title("Sales by region")
    ax.legend()


@case("modern/scatter_colorbar", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    r = rng(11)
    x = r.normal(150)
    y = 0.6 * x + r.normal(150, scale=0.5)
    points = ax.scatter(x, y, c=x + y, s=30)
    fig.colorbar(points, ax=ax, label="x + y")
    ax.set_title("Correlated samples")
    ax.set_xlabel("x")
    ax.set_ylabel("y")


@case("modern/histogram", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    r = rng(12)
    ax.hist(r.normal(800), bins=30, alpha=0.7, label="N(0, 1)")
    ax.hist(r.normal(800, loc=1.5, scale=0.7), bins=30, alpha=0.7, label="N(1.5, 0.7)")
    ax.set_title("Two samples")
    ax.legend()


@case("modern/subplots", figsize=SIZE, rc=MODERN)
def _(fig):
    axs = fig.subplots(2, 2, sharex=True)
    r = rng(13)
    axs[0, 0].plot(X, np.sin(X))
    axs[0, 0].set_title("Line")
    axs[0, 1].scatter(X[::8], np.cos(X[::8]) + r.normal(25, scale=0.1))
    axs[0, 1].set_title("Scatter")
    axs[1, 0].step(X[::10], np.sin(X[::10]) ** 2, where="mid")
    axs[1, 0].set_title("Steps")
    axs[1, 1].fill_between(X, np.sin(X) - 0.3, np.sin(X) + 0.3, alpha=0.3)
    axs[1, 1].plot(X, np.sin(X))
    axs[1, 1].set_title("Band")
    fig.suptitle("Four views of a wave")


@case("modern/image", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    y, x = np.mgrid[-3:3:60j, -3:3:80j]
    field = x * y / (1 + x ** 2 + y ** 2)
    image = ax.imshow(field, extent=(-3, 3, -3, 3), origin="lower")
    ax.grid(False)
    fig.colorbar(image, ax=ax)
    ax.set_title("A field")


@case("modern/polar", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0, 2 * np.pi, 400)
    ax.plot(theta, np.abs(np.cos(3 * theta)))
    ax.set_title("r = |cos 3θ|")


@case("modern/annotations", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 6, 13)
    y = np.cos(2 * x) / (1 + x / 2)
    ax.fill_between(x, y - 0.15, y + 0.15, alpha=0.25)
    ax.errorbar(x, y, yerr=0.15, fmt="o", capsize=3)
    ax.annotate("first trough", xy=(x[3], y[3]), xytext=(2.5, -0.8),
                arrowprops=dict(arrowstyle="->"))
    ax.set_title("Measurements with a band")


@case("modern/log_math", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    x = np.linspace(0, 20, 100)
    ax.semilogy(x, 1 / (1 + x * x * x), label=r"$\frac{1}{1 + x^3}$")
    ax.semilogy(x, 1 / (1 + x ** 2), label=r"$\frac{1}{1 + x^2}$")
    ax.set_title(r"Two decays as $x \to \infty$")
    ax.set_xlabel(r"$x$")
    ax.legend()


def _saddle(n=25):
    """``x y / (1 + x² + y²)`` on an ``n`` × ``n`` grid over [-3, 3]²."""
    xx, yy = np.meshgrid(np.linspace(-3, 3, n), np.linspace(-3, 3, n))
    return xx, yy, xx * yy / (1 + xx ** 2 + yy ** 2)


def _surface(fig):
    ax = fig.add_subplot(projection="3d")
    surface = ax.plot_surface(*_saddle(), cmap="viridis")
    fig.colorbar(surface, ax=ax, shrink=0.6, label="height")
    ax.set_title("A saddle")
    ax.set_xlabel("x")
    ax.set_ylabel("y")
    ax.set_zlabel("z")


@case("modern/surface3d", figsize=SIZE, rc=MODERN)
def _(fig):
    _surface(fig)


@case("modern/dark_surface3d", figsize=SIZE, rc=DARK)
def _(fig):
    _surface(fig)


@case("modern/dark_polar", figsize=SIZE, rc=DARK)
def _(fig):
    ax = fig.add_subplot(projection="polar")
    theta = np.linspace(0, 2 * np.pi, 400)
    ax.plot(theta, np.abs(np.cos(3 * theta)), label="|cos 3θ|")
    ax.plot(theta, np.abs(np.sin(2 * theta)), label="|sin 2θ|")
    ax.set_title("Two roses")
    ax.legend(loc="upper left", bbox_to_anchor=(1.05, 1))


@case("modern/dark_contour", figsize=SIZE, rc=DARK)
def _(fig):
    ax = fig.subplots()
    xx, yy, zz = _saddle(80)
    filled = ax.contourf(xx, yy, zz, levels=10)
    lines = ax.contour(xx, yy, zz, levels=filled.levels[::2], colors=INK_DARK, linewidths=0.8)
    ax.clabel(lines, fontsize=8)
    fig.colorbar(filled, ax=ax, label="height")
    ax.set_title("Contours of a saddle")
    ax.set_xlabel("x")
    ax.set_ylabel("y")


@case("modern/dark_bars", figsize=SIZE, rc=DARK)
def _(fig):
    ax = fig.subplots()
    x = np.arange(4)
    # `plt.ink` draws the error bars in ink; matplotlib draws them black.
    ax.bar(x, [18, 24, 31, 12], 0.6, yerr=[2, 3, 4, 2], capsize=4,
           error_kw={"ecolor": INK_DARK})
    ax.set_xticks(x, ["north", "south", "east", "west"])
    ax.set_ylabel("units (thousands)")
    ax.set_title("Sales with their spread")


@case("modern/table", figsize=SIZE, rc=MODERN)
def _(fig):
    ax = fig.subplots()
    years = {"2023": [18, 24, 31, 12], "2024": [21, 22, 35, 17], "2025": [26, 25, 33, 23]}
    x = np.arange(4)
    for i, (year, values) in enumerate(years.items()):
        ax.bar(x + (i - 1) * 0.25, values, 0.25, label=year)
    table = ax.table(cellText=[[str(v) for v in values] for values in years.values()],
                     rowLabels=list(years), colLabels=["north", "south", "east", "west"],
                     loc="bottom")
    # `plt.table.edgecolor` draws the cells in hairlines; matplotlib in black.
    for cell in table.get_celld().values():
        cell.set_edgecolor(HAIRLINE)
    ax.set_xticks([])
    ax.set_ylabel("units (thousands)")
    ax.set_title("Sales by region")
    ax.legend()
