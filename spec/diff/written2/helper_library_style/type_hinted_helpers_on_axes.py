# Helpers taking Axes and returning artists with type hints and keyword-only options; called on axes of several projections.
from __future__ import annotations

from typing import Optional

import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.axes import Axes
from matplotlib.collections import PathCollection
from matplotlib.lines import Line2D


def scatter_with_fit(ax: Axes, x: np.ndarray, y: np.ndarray, *, degree: int = 1,
                     color: Optional[str] = None) -> tuple[PathCollection, Line2D]:
    sc = ax.scatter(x, y, s=10, color=color)
    coeffs = np.polyfit(x, y, degree)
    xs = np.linspace(x.min(), x.max(), 50)
    (fit,) = ax.plot(xs, np.polyval(coeffs, xs), color=sc.get_facecolor()[0], lw=2)
    fit.set_label(f"deg {degree}: " + ", ".join(f"{c:.2f}" for c in coeffs))
    return sc, fit


def annotate_max(ax: Axes, line: Line2D, *, fmt: str = "{:.2f}") -> None:
    x, y = line.get_data()
    i = int(np.argmax(y))
    ax.annotate(fmt.format(y[i]), (x[i], y[i]), xytext=(5, 5), textcoords="offset points")


x = np.linspace(0, 4, 30)
y = 0.5 * x ** 2 - x + np.cos(5 * x) * 0.3
fig, axs = plt.subplots(1, 3, figsize=(9, 3), dpi=50)
for ax, deg in zip(axs, (1, 2, 3)):
    sc, fit = scatter_with_fit(ax, x, y, degree=deg, color=f"C{deg}")
    annotate_max(ax, fit)
    ax.legend(fontsize=6)
    print(deg, fit.get_label(), len(ax.texts), [round(v, 3) for v in ax.get_ylim()])
pax = fig.add_axes([0.8, 0.6, 0.15, 0.3], projection="polar")
sc, fit = scatter_with_fit(pax, x, y + 3, degree=2)
print(type(pax).__name__, fit.get_label(), np.round(sc.get_facecolor()[0], 3).tolist())
print(scatter_with_fit.__annotations__["return"])
fig.savefig("helpers.png")
