# A registry of plot kinds filled by a decorator, dispatching on a string; each kind draws into a shared axes grid.
from __future__ import annotations

from typing import Callable, Dict

import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.axes import Axes

KINDS: Dict[str, Callable[[Axes, np.ndarray], object]] = {}


def kind(name: str):
    def register(fn):
        KINDS[name] = fn
        fn.kind = name
        return fn
    return register


@kind("line")
def draw_line(ax, data):
    return ax.plot(data)[0]


@kind("hist")
def draw_hist(ax, data):
    return ax.hist(data, bins=5)[2]


@kind("box")
def draw_box(ax, data):
    return ax.boxplot(data)


@kind("violin")
def draw_violin(ax, data):
    return ax.violinplot(data, showmedians=True)


@kind("stem")
def draw_stem(ax, data):
    return ax.stem(data)


data = np.sin(np.arange(40) * 0.7) + np.arange(40) * 0.05
fig, axs = plt.subplots(1, len(KINDS), figsize=(10, 2.5), dpi=50)
results = {}
for ax, (name, fn) in zip(axs, sorted(KINDS.items())):
    out = fn(ax, data)
    ax.set_title(name)
    results[name] = out
    print(name, fn.kind, type(out).__name__, len(ax.get_children()))
print(sorted(results["box"].keys()) if isinstance(results["box"], dict) else None)
print(sorted(results["violin"].keys()))
print(len(results["hist"]), type(results["stem"]).__name__)
fig.savefig("kinds.png")
