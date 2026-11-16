# Read a scatter's per-point face colours (from a colormap) and use them for a matching bar chart and legend.
import numpy as np
import matplotlib.pyplot as plt

values = np.array([3.0, 7.0, 5.0, 9.0, 1.0])
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
sc = a.scatter(range(5), values, c=values, cmap="plasma", s=120)
fig.canvas.draw()
colors = sc.get_facecolors()
print(colors.shape, np.round(colors[0], 4).tolist())
bars = b.bar(range(5), values, color=colors)
for bar, c in zip(bars, colors):
    bar.set_edgecolor(np.clip(c[:3] * 0.6, 0, 1))
print([np.round(bar.get_facecolor(), 4).tolist() for bar in bars][:2])
print(sc.get_array().tolist(), sc.get_clim())
fig.savefig("shared_colors.png", dpi=50)
