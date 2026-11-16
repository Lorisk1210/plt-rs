# Alternate shaded bands between major ticks: read tick locations after draw and axvspan every other interval.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 37, 300)
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(x, np.sin(x / 3) * x)
fig.canvas.draw()
ticks = ax.get_xticks()
lo, hi = ax.get_xlim()
for a_, b_ in zip(ticks[::2], ticks[1::2]):
    ax.axvspan(max(a_, lo), min(b_, hi), color="0.92", zorder=0)
print(ticks.tolist(), len(ax.patches), [round(v, 3) for v in ax.get_xlim()])
fig.savefig("bands.png", dpi=50)
