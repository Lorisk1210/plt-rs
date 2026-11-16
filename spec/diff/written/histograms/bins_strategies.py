# Compare bin strategies ('auto', 'fd', 'sturges', explicit edges) on the same skewed sample.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(18)
data = rng.lognormal(0, 0.6, 700)
fig, axs = plt.subplots(2, 2, figsize=(8, 6), sharex=True)
for ax, bins in zip(axs.flat, ["auto", "fd", "sturges", [0, 0.5, 1, 1.5, 2, 3, 5, 8]]):
    n, edges, _ = ax.hist(data, bins=bins, color="slateblue", edgecolor="w")
    ax.set_title(f"bins={bins if isinstance(bins, str) else 'edges'} ({len(n)})")
    print(len(n))
fig.tight_layout()
fig.savefig("bins.png")
fig.suptitle("Bin strategies on a log-normal sample")
fig.savefig("bins_titled.png", dpi=60)
