# Simulate 1000 rolls of two dice and plot the histogram of sums with integer bins.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(42)
sums = rng.integers(1, 7, 1000) + rng.integers(1, 7, 1000)
plt.hist(sums, bins=np.arange(1.5, 13.5, 1), edgecolor="black", rwidth=0.9)
plt.xticks(range(2, 13))
plt.xlabel("sum of two dice")
plt.ylabel("how often")
plt.title("1000 rolls")
plt.savefig("dice.png")
print(np.bincount(sums)[2:].tolist())
