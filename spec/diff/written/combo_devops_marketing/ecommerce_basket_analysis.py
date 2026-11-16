# Basket analysis: co-occurrence heat map of products, basket size histogram, and top pairs bar with lift values.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(205)
products = ["milk", "bread", "eggs", "butter", "jam", "coffee"]
p = np.array([0.5, 0.45, 0.3, 0.25, 0.15, 0.35])
baskets = rng.random((1000, 6)) < p
baskets[:, 3] |= baskets[:, 1] & (rng.random(1000) < 0.3)
co = baskets.T.astype(int) @ baskets.astype(int)
support = baskets.mean(axis=0)
lift = (co / 1000) / np.outer(support, support)
np.fill_diagonal(lift, np.nan)
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
im = axs[0].imshow(lift, cmap="PuOr", vmin=0.5, vmax=1.5)
axs[0].set_xticks(range(6), products, rotation=45); axs[0].set_yticks(range(6), products)
fig.colorbar(im, ax=axs[0], label="lift")
axs[1].hist(baskets.sum(axis=1), bins=np.arange(8) - 0.5)
pairs = sorted(((lift[i, j], products[i], products[j]) for i in range(6) for j in range(i + 1, 6)), reverse=True)[:5]
axs[2].barh([f"{a}+{b}" for _, a, b in pairs], [l for l, _, _ in pairs])
axs[2].invert_yaxis()
fig.tight_layout()
print([(a, b, round(float(l), 3)) for l, a, b in pairs])
fig.savefig("baskets.png", dpi=50)
