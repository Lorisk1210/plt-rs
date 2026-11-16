# Notebook-like flow: one figure created early, later "cells" add layers to its axes, inspect, and finally save.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(7, 4))
x = np.linspace(0, 10, 200)
ax.plot(x, np.sin(x), label="signal")
print("cell 1", len(ax.lines))
noise = np.random.default_rng(241).normal(0, 0.2, x.size)
ax.plot(x, np.sin(x) + noise, ".", ms=2, alpha=0.4, label="noisy")
print("cell 2", len(ax.lines), [round(v, 3) for v in ax.get_ylim()])
smooth = np.convolve(np.sin(x) + noise, np.ones(15) / 15, mode="same")
ax.plot(x, smooth, lw=2, label="smoothed")
ax.legend()
print("cell 3", [t.get_text() for t in ax.get_legend().get_texts()])
ax.set_xlim(2, 8)
ax.set_title("filtering")
fig.tight_layout()
print("cell 4", ax.get_xlim(), ax.get_title())
fig.savefig("notebook_flow.png", dpi=50)
