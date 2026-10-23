"""Time the benchmark workloads of ``cargo xtask bench`` in matplotlib.

    .venv/bin/python spec/reference/bench/bench.py [--runs N] NAME ...

Each workload mirrors the plt-rs one of the same name in
``xtask/src/bench/workloads``: the same data (``spec_support``'s SplitMix64
stream, vectorised here so that making ten million samples stays quick),
the same figure, saved to memory at 100 dpi under the reference
configuration. The data is made before the clock starts. One line per
workload is printed: its name, a tab, and its best time in milliseconds.
"""

from __future__ import annotations

import argparse
import gc
import io
import math
import pathlib
import sys
import time
from typing import Callable, NamedTuple

import numpy as np

REFERENCE = pathlib.Path(__file__).resolve().parent.parent
DPI = 100

Build = Callable[["matplotlib.figure.Figure"], object]


class FastRng:
    """``spec_support.SpecRng``'s stream, drawn with numpy in bulk."""

    _GOLDEN = np.uint64(0x9E3779B97F4A7C15)

    def __init__(self, seed: int) -> None:
        self._seed = np.uint64(seed)
        self._drawn = 0

    def _next_f64(self, n: int) -> np.ndarray:
        k = np.arange(self._drawn + 1, self._drawn + n + 1, dtype=np.uint64)
        self._drawn += n
        z = self._seed + k * self._GOLDEN
        z = (z ^ (z >> np.uint64(30))) * np.uint64(0xBF58476D1CE4E5B9)
        z = (z ^ (z >> np.uint64(27))) * np.uint64(0x94D049BB133111EB)
        z ^= z >> np.uint64(31)
        return (z >> np.uint64(11)).astype(np.float64) * (1.0 / (1 << 53))

    def uniform(self, n: int, low: float, high: float) -> np.ndarray:
        return low + (high - low) * self._next_f64(n)

    def normal(self, n: int, loc: float, scale: float) -> np.ndarray:
        """Box–Muller, two uniforms per sample, as ``SpecRng.normal``."""
        u = self._next_f64(2 * n)
        u1, u2 = 1.0 - u[0::2], u[1::2]
        return loc + scale * np.sqrt(-2.0 * np.log(u1)) * np.cos(2.0 * math.pi * u2)


def random_walk(n: int, seed: int) -> tuple[np.ndarray, np.ndarray]:
    return np.arange(n, dtype=float), np.cumsum(FastRng(seed).normal(n, 0.0, 1.0))


def smooth_field(n: int) -> np.ndarray:
    t = np.linspace(-10.0, 10.0, n)
    return np.sin(t)[None, :] * np.cos(t)[:, None] + t[None, :] / 10.0


def correlated_normals(seed: int) -> tuple[np.ndarray, np.ndarray]:
    rng = FastRng(seed)
    x = rng.normal(1_000_000, 0.0, 1.0)
    return x, 0.5 * x + rng.normal(1_000_000, 0.0, 1.0)


def normal_groups(groups: int, seed: int) -> list[np.ndarray]:
    rng = FastRng(seed)
    return [rng.normal(1000, g / 10.0, 1.0 + g / 100.0) for g in range(groups)]


def swirl(n: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    t = np.linspace(-3.0, 3.0, n)
    x, y = np.meshgrid(t, t)
    return t, -1.0 - x * x + y, 1.0 + x - y * y


def walk_1e6() -> Build:
    x, y = random_walk(1_000_000, 1)
    return lambda fig: fig.subplots().plot(x, y)


def many_lines() -> Build:
    x = np.linspace(0.0, 10.0, 1000)
    waves = [np.sin(x * (1.0 + k / 1000.0)) + k / 100.0 for k in range(1000)]

    def build(fig):
        ax = fig.subplots()
        for wave in waves:
            ax.plot(x, wave)

    return build


def dashed() -> Build:
    x, y = random_walk(100_000, 2)
    return lambda fig: fig.subplots().plot(x, y, "--")


def markevery() -> Build:
    x, y = random_walk(100_000, 3)
    return lambda fig: fig.subplots().plot(x, y, "-o", markevery=100)


def scatter() -> Build:
    rng = FastRng(4)
    x, y = rng.uniform(100_000, 0.0, 1.0), rng.uniform(100_000, 0.0, 1.0)
    sizes, values = rng.uniform(100_000, 5.0, 50.0), rng.uniform(100_000, 0.0, 1.0)
    return lambda fig: fig.subplots().scatter(x, y, s=sizes, c=values)


def plot_markers() -> Build:
    rng = FastRng(5)
    x, y = rng.uniform(100_000, 0.0, 1.0), rng.uniform(100_000, 0.0, 1.0)
    return lambda fig: fig.subplots().plot(x, y, "o")


def pcolormesh() -> Build:
    z = smooth_field(1000)
    return lambda fig: fig.subplots().pcolormesh(z)


def imshow(interpolation: str) -> Callable[[], Build]:
    def setup() -> Build:
        z = smooth_field(4000)
        return lambda fig: fig.subplots().imshow(z, interpolation=interpolation)

    return setup


def hexbin() -> Build:
    x, y = correlated_normals(6)
    return lambda fig: fig.subplots().hexbin(x, y)


def hist2d() -> Build:
    x, y = correlated_normals(7)
    return lambda fig: fig.subplots().hist2d(x, y, bins=100)


def hist() -> Build:
    values = FastRng(8).normal(10_000_000, 0.0, 1.0)
    return lambda fig: fig.subplots().hist(values, bins=1000)


def boxplot() -> Build:
    groups = normal_groups(100, 9)
    return lambda fig: fig.subplots().boxplot(groups)


def violinplot() -> Build:
    groups = normal_groups(50, 10)
    return lambda fig: fig.subplots().violinplot(groups)


def contours(filled: bool) -> Callable[[], Build]:
    def setup() -> Build:
        z = smooth_field(1000)
        if filled:
            return lambda fig: fig.subplots().contourf(z, levels=20)
        return lambda fig: fig.subplots().contour(z, levels=20)

    return setup


def streamplot() -> Build:
    t, u, v = swirl(100)
    return lambda fig: fig.subplots().streamplot(t, t, u, v)


def quiver() -> Build:
    t, u, v = swirl(100)
    x, y = np.meshgrid(t, t)
    return lambda fig: fig.subplots().quiver(x, y, u, v)


def tricontourf() -> Build:
    rng = FastRng(11)
    x, y = rng.uniform(100_000, -3.0, 3.0), rng.uniform(100_000, -3.0, 3.0)
    z = np.exp(-x * x - y * y) + 0.3 * np.sin(2.0 * x)
    return lambda fig: fig.subplots().tricontourf(x, y, z)


def plot_surface() -> Build:
    t = np.linspace(-5.0, 5.0, 200)
    x, y = np.meshgrid(t, t)
    z = np.sin(np.hypot(x, y))
    return lambda fig: fig.add_subplot(projection="3d").plot_surface(x, y, z, cmap="viridis")


def scatter_3d() -> Build:
    rng = FastRng(12)
    x, y, z = (rng.uniform(10_000, 0.0, 1.0) for _ in range(3))
    return lambda fig: fig.add_subplot(projection="3d").scatter(x, y, z)


def subplots() -> Build:
    x = np.linspace(0.0, 10.0, 100)

    def build(fig):
        for k, ax in enumerate(fig.subplots(10, 10).flat):
            ax.plot(x, np.sin(x + k / 10.0))
            ax.set_title(f"panel {k}")
            ax.set_xlabel("x")
            ax.set_ylabel("y")

    return build


def texts() -> Build:
    def build(fig):
        ax = fig.subplots()
        for i in range(2000):
            ax.text((i % 50) / 50.0, (i // 50) / 40.0, f"t{i}", fontsize=6)

    return build


def mathtext() -> Build:
    def build(fig):
        ax = fig.subplots()
        for k in range(50):
            formula = (rf"$\sum_{{n=1}}^{{{k}}} \frac{{x^{{n}}}}{{n!}} + "
                       rf"\sqrt{{\alpha_{{{k}}}}} - \int_0^{{{k}}} \beta\,dt$")
            ax.text((k % 5) / 5.0, (k // 5) / 10.0, formula)

    return build


def constrained() -> Build:
    image = np.fromfunction(lambda i, j: np.sin(i * j / 40.0), (20, 20))

    def build(fig):
        fig.set_layout_engine("constrained")
        for k, ax in enumerate(fig.subplots(6, 6).flat):
            fig.colorbar(ax.imshow(image))
            ax.set_title(f"image {k}")

    return build


def mid_size() -> Build:
    rng = FastRng(13)
    x = np.linspace(0.0, 10.0, 2000)
    scatter_x, scatter_y = rng.uniform(2000, 0.0, 1.0), rng.uniform(2000, 0.0, 1.0)
    heights = rng.uniform(30, 1.0, 10.0)
    sample = rng.normal(10_000, 0.0, 1.0)

    def build(fig):
        axs = fig.subplots(2, 2)
        lines = axs[0, 0]
        for k in range(3):
            lines.plot(x, np.sin(x + k) * np.exp(-x / 10.0), label=f"phase {k}")
        lines.set_title("damped waves")
        lines.set_xlabel("time")
        lines.set_ylabel("amplitude")
        lines.legend()
        axs[0, 1].scatter(scatter_x, scatter_y, c=scatter_y)
        axs[1, 0].bar(np.arange(len(heights), dtype=float), heights)
        axs[1, 1].hist(sample, bins=50)
        axs[1, 1].set_title(r"$\mathcal{N}(0, 1)$: $\frac{1}{\sqrt{2\pi}} e^{-x^2/2}$")

    return build


class Workload(NamedTuple):
    setup: Callable[[], Build]
    format: str = "png"
    figsize: tuple[float, float] = (6.4, 4.8)


WORKLOADS = {
    "lines/plot_1e6_png": Workload(walk_1e6),
    "lines/plot_1e6_svg": Workload(walk_1e6, "svg"),
    "lines/plot_1e6_pdf": Workload(walk_1e6, "pdf"),
    "lines/1000_lines_of_1000": Workload(many_lines),
    "lines/dashed_1e5": Workload(dashed),
    "lines/markevery_1e5": Workload(markevery),
    "markers/scatter_1e5": Workload(scatter),
    "markers/plot_o_1e5": Workload(plot_markers),
    "meshes/pcolormesh_1000": Workload(pcolormesh),
    "meshes/imshow_4000_antialiased": Workload(imshow("antialiased")),
    "meshes/imshow_4000_nearest": Workload(imshow("nearest")),
    "meshes/hexbin_1e6": Workload(hexbin),
    "meshes/hist2d_1e6": Workload(hist2d),
    "stats/hist_1e7": Workload(hist),
    "stats/boxplot_100": Workload(boxplot),
    "stats/violinplot_50": Workload(violinplot),
    "fields/contour_1000": Workload(contours(False)),
    "fields/contourf_1000": Workload(contours(True)),
    "fields/streamplot_100": Workload(streamplot),
    "fields/quiver_100": Workload(quiver),
    "fields/tricontourf_1e5": Workload(tricontourf),
    "3d/plot_surface_200": Workload(plot_surface),
    "3d/scatter_1e4": Workload(scatter_3d),
    "text/subplots_10x10": Workload(subplots, figsize=(12.8, 9.6)),
    "text/text_2000": Workload(texts),
    "text/mathtext_50": Workload(mathtext),
    "text/constrained_6x6_colorbars": Workload(constrained, figsize=(14.0, 12.0)),
    "output/mid_png": Workload(mid_size),
    "output/mid_svg": Workload(mid_size, "svg"),
    "output/mid_pdf": Workload(mid_size, "pdf"),
    "output/mid_ps": Workload(mid_size, "ps"),
    "output/mid_pgf": Workload(mid_size, "pgf"),
    "output/mid_eps": Workload(mid_size, "eps"),
}


def best_ms(workload: Workload, runs: int) -> float:
    """The fastest of ``runs`` builds and saves, in milliseconds."""
    from matplotlib.figure import Figure

    build = workload.setup()
    best = math.inf
    for _ in range(runs):
        gc.collect()
        start = time.perf_counter()
        fig = Figure(figsize=workload.figsize, dpi=DPI)
        build(fig)
        fig.savefig(io.BytesIO(), format=workload.format, dpi=DPI)
        best = min(best, time.perf_counter() - start)
    return best * 1e3


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("names", nargs="*", help="workloads to time (default: all)")
    parser.add_argument("--runs", type=int, default=3, help="runs per workload (default: 3)")
    args = parser.parse_args()
    unknown = [name for name in args.names if name not in WORKLOADS]
    if unknown:
        parser.error(f"unknown workloads: {', '.join(unknown)}")
    sys.path.insert(0, str(REFERENCE))
    import matplotlib

    matplotlib.use("Agg")
    from conftest import reference_rc
    from figures.render_vectors import _measure_pgf_text_like_agg

    reference_rc()
    _measure_pgf_text_like_agg()
    for name in args.names or WORKLOADS:
        print(f"{name}\t{best_ms(WORKLOADS[name], args.runs):.3f}", flush=True)


if __name__ == "__main__":
    main()
