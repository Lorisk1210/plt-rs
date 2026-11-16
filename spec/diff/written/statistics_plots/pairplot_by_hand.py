# Pair plot of three variables: scatter off-diagonal, histograms on the diagonal, coloured by group.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
groups = np.repeat([0, 1], 60)
data = np.c_[rng.normal(groups * 2, 1), rng.normal(groups, 0.7), rng.normal(-groups, 1.2)]
names = ["x1", "x2", "x3"]
fig, axs = plt.subplots(3, 3, figsize=(7, 7))
for i in range(3):
    for j in range(3):
        ax = axs[i, j]
        for g, c in [(0, "C0"), (1, "C1")]:
            if i == j:
                ax.hist(data[groups == g, i], bins=12, alpha=0.6, color=c)
            else:
                ax.scatter(data[groups == g, j], data[groups == g, i], s=6, color=c)
        if i == 2:
            ax.set_xlabel(names[j])
        if j == 0:
            ax.set_ylabel(names[i])
fig.tight_layout()
print(np.round(np.corrcoef(data.T)[0, 1], 4))
fig.savefig("pairs.png")
