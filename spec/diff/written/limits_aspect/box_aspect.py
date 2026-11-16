# Square axes box regardless of data range with set_box_aspect, for a parity plot.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(13)
true = rng.uniform(0, 100, 80)
pred = true + rng.normal(0, 8, 80)
fig, ax = plt.subplots(figsize=(6, 4))
ax.scatter(true, pred, s=12)
lims = [min(true.min(), pred.min()), max(true.max(), pred.max())]
ax.plot(lims, lims, "r--")
ax.set_box_aspect(1)
ax.set_xlabel("true")
ax.set_ylabel("predicted")
rmse = np.sqrt(np.mean((pred - true) ** 2))
ax.text(0.05, 0.92, f"RMSE {rmse:.2f}", transform=ax.transAxes)
print(ax.get_box_aspect(), round(rmse, 4))
fig.savefig("parity.png")
