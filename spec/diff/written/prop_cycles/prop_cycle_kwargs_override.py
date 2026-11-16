# Explicit keywords override the cycle but do not consume it (does colour advance? users check).
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
l1, = ax.plot([1, 2], [1, 2])
l2, = ax.plot([1, 2], [2, 3], color="k")
l3, = ax.plot([1, 2], [3, 4])
l4, = ax.plot([1, 2], [4, 5], "r--")
l5, = ax.plot([1, 2], [5, 6])
print([l.get_color() for l in (l1, l2, l3, l4, l5)])
sc = ax.scatter([1.5], [2.5])
sc2 = ax.scatter([1.5], [3.5], c="k")
sc3 = ax.scatter([1.5], [4.5])
print([s.get_facecolor().round(3).tolist() for s in (sc, sc2, sc3)])
fig.savefig("override.png")
