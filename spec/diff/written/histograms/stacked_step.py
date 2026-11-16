# Stacked histogram of three groups and an unfilled step histogram of their total.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
groups = [rng.exponential(s, 300) for s in (1.0, 2.0, 3.0)]
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
n, bins, _ = a.hist(groups, bins=20, stacked=True, label=["fast", "medium", "slow"], range=(0, 12))
a.legend()
a.set_title("stacked")
b.hist(np.concatenate(groups), bins=bins, histtype="step", lw=2, color="k", label="total")
b.hist(groups, bins=bins, histtype="step", label=["fast", "medium", "slow"])
b.legend()
b.set_title("step")
print(np.array(n).shape, n[-1][:3])
fig.savefig("waits.png")
