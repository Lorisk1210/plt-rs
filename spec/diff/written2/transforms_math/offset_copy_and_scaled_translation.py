# Labels offset by points from data points with offset_copy and ScaledTranslation; verify display positions after draw.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
x = np.arange(5)
y = x ** 2
ax.plot(x, y, "o")
off = mtransforms.offset_copy(ax.transData, fig=fig, x=6, y=-12, units="points")
off_px = mtransforms.offset_copy(ax.transData, fig=fig, x=10, y=10, units="dots")
off_in = mtransforms.offset_copy(ax.transData, fig=fig, x=0.1, y=0.0, units="inches")
for xi, yi in zip(x, y):
    ax.text(xi, yi, f"{yi}", transform=off, fontsize=8)
scaled = ax.transData + mtransforms.ScaledTranslation(5 / 72, 5 / 72, fig.dpi_scale_trans)
fig.canvas.draw()
p = (2, 4)
base = ax.transData.transform(p)
for name, t in [("points", off), ("dots", off_px), ("inches", off_in), ("scaled", scaled)]:
    print(name, type(t).__name__, np.round(t.transform(p) - base, 6).tolist())
fig.set_dpi(200)
print("dpi200", np.round(off.transform(p) - ax.transData.transform(p), 6).tolist(),
      np.round(scaled.transform(p) - ax.transData.transform(p), 6).tolist())
try:
    mtransforms.offset_copy(ax.transData, fig=fig, units="furlongs")
except ValueError as err:
    print("ValueError:", err)
print(len(ax.texts))
fig.savefig("offsets.png")
