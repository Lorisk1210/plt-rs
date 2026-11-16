# Central limit theorem lesson: sample-mean histograms for growing n with the normal curve, plus SE vs n on log-log.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(171)
pop = rng.exponential(1, 100000)
ns = [1, 2, 5, 30]
fig = plt.figure(figsize=(12, 7))
for k, n in enumerate(ns):
    ax = fig.add_subplot(2, 4, k + 1)
    means = rng.choice(pop, (2000, n)).mean(axis=1)
    ax.hist(means, bins=40, density=True, color="C0", alpha=0.7)
    x = np.linspace(means.min(), means.max(), 200)
    s = 1 / np.sqrt(n)
    ax.plot(x, np.exp(-(x - 1) ** 2 / (2 * s ** 2)) / (s * np.sqrt(2 * np.pi)), "r")
    ax.set_title(f"n = {n}")
ax = fig.add_subplot(2, 1, 2)
grid = np.array([2, 5, 10, 20, 50, 100, 200])
se = [rng.choice(pop, (1000, n)).mean(axis=1).std() for n in grid]
ax.loglog(grid, se, "o-", label="simulated")
ax.loglog(grid, 1 / np.sqrt(grid), "k--", label=r"$1/\sqrt{n}$")
ax.legend()
fig.tight_layout()
print([round(float(v), 4) for v in se])
fig.savefig("clt.png", dpi=50)
