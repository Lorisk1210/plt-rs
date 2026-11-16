# Black-and-white stacked area chart using hatches and a legend that shows them.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(10)
ys = [np.full(10, 2.0), 1 + np.arange(10) * 0.2, 3 - np.arange(10) * 0.1]
fig, ax = plt.subplots(figsize=(6, 4))
polys = ax.stackplot(x, ys, labels=["base", "growth", "decline"], colors=["white"] * 3, edgecolor="k")
for p, h in zip(polys, ["", "///", "..."]):
    p.set_hatch(h)
ax.legend(loc="upper left")
print([p.get_hatch() for p in polys])
fig.savefig("bw_stack.png")
