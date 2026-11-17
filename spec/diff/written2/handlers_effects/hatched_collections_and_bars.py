# Hatching on collections: bar containers, fill_between, contourf hatches, PatchCollection with per-patch hatch via set_hatch.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PatchCollection
from matplotlib.patches import Wedge

fig, axs = plt.subplots(2, 2, figsize=(6, 5))
bars = axs[0, 0].bar(range(4), [3, 1, 4, 1], hatch=["/", "\\", "x", "o"], fill=False)
print([b.get_hatch() for b in bars])
fb = axs[0, 1].fill_between(np.linspace(0, 1, 20), 0, np.linspace(0, 1, 20) ** 2, hatch="..", facecolor="none", edgecolor="C1")
print(type(fb).__name__, fb.get_hatch())
X, Y = np.meshgrid(np.linspace(-2, 2, 40), np.linspace(-2, 2, 40))
cs = axs[1, 0].contourf(X, Y, np.exp(-X**2 - Y**2), levels=4, hatches=["", "/", "//", "x"], cmap="gray", alpha=0.5)
print(cs.hatches, len(cs.get_paths()))
wedges = [Wedge((0.5, 0.5), 0.4, k * 60, (k + 1) * 60) for k in range(6)]
pc = PatchCollection(wedges, facecolor="none", edgecolor="k")
pc.set_hatch("++")
axs[1, 1].add_collection(pc)
print(pc.get_hatch(), len(pc.get_paths()))
fig.canvas.draw()
fig.savefig("hatched.png")
hb = axs[0, 0].barh([5], [2], hatch="*", color="C4")
print(hb[0].get_hatch(), hb[0].get_facecolor())
pc.set_hatch(None)
print(repr(pc.get_hatch()))
fig.savefig("hatched2.png")
for b, h in zip(bars, ["-", "|", "+", "."]):
    b.set_hatch(h * 2)
print([b.get_hatch() for b in bars], [b.get_hatch_linewidth() for b in bars][:2])
fig.savefig("hatched3.png")
