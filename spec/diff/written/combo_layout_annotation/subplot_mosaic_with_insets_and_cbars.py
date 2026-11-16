# Mosaic with nested layout, two insets, a shared colour bar for two image panels and a figure legend.
import numpy as np
import matplotlib.pyplot as plt

fig, axd = plt.subplot_mosaic([["img1", "img2", "plot"], ["img1", "img2", "hist"]], figsize=(12, 6), layout="constrained")
y, x = np.mgrid[0:1:40j, 0:1:40j]
im1 = axd["img1"].imshow(np.sin(6 * x) * y, vmin=-1, vmax=1, cmap="RdBu")
im2 = axd["img2"].imshow(np.cos(5 * y) * x, vmin=-1, vmax=1, cmap="RdBu")
fig.colorbar(im1, ax=[axd["img1"], axd["img2"]], location="bottom", shrink=0.6)
l1, = axd["plot"].plot(np.arange(10), label="a")
l2, = axd["plot"].plot(np.arange(10)[::-1], label="b")
ins = axd["plot"].inset_axes([0.6, 0.6, 0.35, 0.35])
ins.plot(np.arange(10) ** 2, color="C2")
axd["hist"].hist(np.random.default_rng(120).normal(size=200), bins=15)
ins2 = axd["hist"].inset_axes([0.65, 0.55, 0.3, 0.35])
ins2.boxplot(np.random.default_rng(121).normal(size=50))
fig.legend([l1, l2], ["a", "b"], loc="outside upper center", ncols=2)
print(sorted(axd), len(fig.axes))
fig.savefig("mosaic_insets.png", dpi=50)
