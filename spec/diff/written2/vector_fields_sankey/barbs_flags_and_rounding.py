# Barbs with speeds needing flags, full and half barbs, rounding on/off, increments and sizes; per-barb vertex counts.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

speeds = np.array([0, 3, 5, 7, 10, 15, 25, 48, 50, 65, 103])
x = np.arange(len(speeds))
fig, axs = plt.subplots(3, 1, figsize=(8, 5), dpi=50)
variants = [dict(), dict(rounding=False), dict(barb_increments=dict(half=10, full=20, flag=100), length=8,
                                              sizes=dict(emptybarb=0.4, spacing=0.2, height=0.6), fill_empty=True)]
for ax, kw in zip(axs, variants):
    b = ax.barbs(x, np.zeros_like(x), speeds, np.zeros_like(speeds), **kw)
    ax.set_xlim(-1, len(speeds))
    fig.canvas.draw()
    print(sorted(kw), [len(p.vertices) for p in b.get_paths()])
b = axs[0].barbs([0], [1], [-30], [30], flip_barb=True, pivot="middle", barbcolor="r", flagcolor="b")
fig.canvas.draw()
print(len(b.get_paths()[0].vertices), np.round(b.get_edgecolor()[0], 3).tolist(), np.round(b.get_facecolor()[0], 3).tolist())
b.set_UVC([10], [0])
b.set_offsets(np.array([[2.0, 2.0]]))
fig.canvas.draw()
print(len(b.get_paths()[0].vertices), b.get_offsets().tolist())
fig.savefig("barbs.png")
bc = axs[1].barbs(x, np.ones_like(x), speeds, speeds, speeds, cmap="viridis")
fig.canvas.draw()
print(bc.get_array().tolist()[:4], np.round(bc.get_facecolor()[1], 3).tolist())
print(bc.sizes, bc.barb_increments, bc.rounding, bc.fill_empty)
bc.set_UVC(speeds * 2, speeds * 0, speeds * 2)
fig.canvas.draw()
print([len(p.vertices) for p in bc.get_paths()][-3:], bc.get_array().tolist()[-2:])
