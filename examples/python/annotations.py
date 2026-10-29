"""Annotations and arrows: a year of a share price with its high, its
sharpest fall and a summary box pointed out."""

import numpy as np

import pltrs.pyplot as plt
from samples import Samples

returns = Samples(21).normal(250, 0.0012, 0.014)
returns[170] -= 0.11  # a profit warning
price = 100 * np.exp(np.cumsum(returns))
day = np.arange(250)
high = np.argmax(price)
fall = np.argmin(np.diff(price)) + 1

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
ax.plot(day, price, linewidth=1.5)
ax.annotate(
    f"high: {price[high]:.1f}",
    xy=(high, price[high]),
    xytext=(25, 12),
    textcoords="offset points",
    arrowprops=dict(arrowstyle="->", connectionstyle="arc3,rad=0.3"),
)
ax.annotate(
    "profit warning",
    xy=(fall, price[fall]),
    xytext=(-120, -8),
    textcoords="offset points",
    color="tab:red",
    bbox=dict(boxstyle="round,pad=0.4", facecolor="white", edgecolor="tab:red"),
    arrowprops=dict(arrowstyle="-|>", color="tab:red", linewidth=1.5),
)
change = price[-1] / price[0] - 1
ax.text(
    0.03,
    0.95,
    f"close {price[-1]:.1f}\nyear {change:+.1%}",
    transform=ax.transAxes,
    va="top",
    bbox=dict(boxstyle="round,pad=0.5", facecolor="0.96", edgecolor="0.8"),
)
ax.set_title("Share price over one year")
ax.set_xlabel("trading day")
ax.set_ylabel("price (€)")
ax.set_xlim(0, 249)
ax.set_ylim(75, 115)

fig.savefig("annotations.png")
