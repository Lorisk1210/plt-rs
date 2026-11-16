# A simple dendrogram drawn by hand from a fixed merge table with U-shaped links.
import matplotlib.pyplot as plt

leaves = ["a", "b", "c", "d", "e"]
merges = [(0, 1, 1.0), (2, 3, 1.5), (5, 4, 2.5), (6, 7, 4.0)]
pos = {i: (float(i), 0.0) for i in range(len(leaves))}
fig, ax = plt.subplots(figsize=(6, 3.5))
for k, (i, j, h) in enumerate(merges):
    (xi, yi), (xj, yj) = pos[i], pos[j]
    ax.plot([xi, xi, xj, xj], [yi, h, h, yj], "k-")
    pos[len(leaves) + k] = ((xi + xj) / 2, h)
ax.set_xticks(range(len(leaves)), leaves)
ax.set_ylabel("distance")
ax.spines[["top", "right"]].set_visible(False)
print(pos[len(leaves) + len(merges) - 1])
fig.savefig("dendrogram.png")
