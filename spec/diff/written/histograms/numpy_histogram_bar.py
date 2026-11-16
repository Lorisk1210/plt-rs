# Compute the histogram with numpy and draw it with bar() and stairs() instead of hist().
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(40)
ages = rng.integers(18, 80, 1000)
counts, edges = np.histogram(ages, bins=range(15, 85, 5))
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.bar(edges[:-1], counts, width=5, align="edge", edgecolor="white")
a.set_title("bar")
b.stairs(counts, edges, fill=True, alpha=0.4)
b.stairs(counts, edges, color="k")
b.set_title("stairs")
print(counts.tolist())
fig.savefig("ages.png")
