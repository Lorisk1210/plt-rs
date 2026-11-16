# Hatches in pie wedges and histogram bars and contourf regions in one figure.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(11, 3.5))
axs[0].pie([3, 2, 1], hatch=["**", "o", "\\\\"], colors=["w"] * 3, wedgeprops=dict(edgecolor="k"))
n, b, patches = axs[1].hist(np.random.default_rng(1).normal(size=300), bins=10, color="w", edgecolor="k")
for p, h in zip(patches, ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*"]):
    p.set_hatch(h)
X, Y = np.meshgrid(np.linspace(-2, 2, 50), np.linspace(-2, 2, 50))
axs[2].contourf(X, Y, X * Y, levels=4, hatches=["", "..", "//", "xx"], cmap="Greys", alpha=0.6)
print([p.get_hatch() for p in patches][:4])
fig.savefig("hatch_mix.png")
