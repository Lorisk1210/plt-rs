"""Log axes: how the running time of three sorting algorithms grows with
the input, power laws becoming straight lines on log-log axes."""

import numpy as np

import pltrs.pyplot as plt

n = np.logspace(1, 7, 13)
algorithms = {
    "insertion sort, O(n²)": 2.5e-9 * n**2,
    "merge sort, O(n log n)": 4e-8 * n * np.log2(n),
    "radix sort, O(n)": 1.2e-7 * n,
}

fig, ax = plt.subplots(figsize=(7.2, 4.5), layout="constrained")
for (label, seconds), marker in zip(algorithms.items(), ["o", "s", "^"]):
    ax.loglog(n, seconds, marker=marker, label=label)
ax.axhline(1, color="0.5", linestyle=":", linewidth=1)
ax.text(12, 1.6, "one second", color="0.4")
ax.grid(which="minor", linewidth=0.4, alpha=0.5)
ax.set_title("Running time of sorting algorithms")
ax.set_xlabel("number of items")
ax.set_ylabel("time (s)")
ax.set_ylim(1e-7, 1e5)
ax.legend()

fig.savefig("log_axes.png")
