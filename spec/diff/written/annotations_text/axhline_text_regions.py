# Thresholds with axhline/axvline and shaded regions labelled with text inside the axes.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(48)
glucose = 100 + 40 * np.sin(x / 6) + 5 * np.cos(x)
fig, ax = plt.subplots(figsize=(7, 3.5))
ax.plot(x, glucose, "k")
ax.axhspan(70, 140, color="green", alpha=0.1)
ax.axhline(180, color="red", ls="--")
ax.axhline(70, color="orange", ls=":")
ax.axvline(24, color="gray", lw=0.8)
ax.text(1, 182, "high", color="red", va="bottom")
ax.text(1, 72, "low", color="orange", va="bottom")
ax.text(24.5, ax.get_ylim()[0] + 5, "midnight", rotation=90, va="bottom", fontsize=8)
ax.set_xlabel("hour")
print(np.round(ax.get_ylim(), 3))
fig.savefig("glucose.png")
