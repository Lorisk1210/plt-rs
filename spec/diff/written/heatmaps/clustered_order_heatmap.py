# Reorder rows and columns of a matrix by their means before plotting (manual "clustering").
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
genes = [f"g{i}" for i in range(12)]
samples = [f"s{j}" for j in range(8)]
expr = rng.normal(0, 1, (12, 8)) + np.linspace(-2, 2, 12)[:, None] * np.linspace(-1, 1, 8)[None, :]
row_order = np.argsort(expr.mean(axis=1))
col_order = np.argsort(expr.mean(axis=0))
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 4))
a.imshow(expr, cmap="bwr", vmin=-3, vmax=3, aspect="auto")
a.set_title("raw order")
im = b.imshow(expr[row_order][:, col_order], cmap="bwr", vmin=-3, vmax=3, aspect="auto")
b.set_yticks(range(12), [genes[i] for i in row_order], fontsize=7)
b.set_xticks(range(8), [samples[j] for j in col_order], fontsize=7)
b.set_title("sorted by mean")
fig.colorbar(im, ax=[a, b], shrink=0.8)
print(row_order.tolist())
fig.savefig("sorted.png")
