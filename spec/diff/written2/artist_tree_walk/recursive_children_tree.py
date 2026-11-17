# Walk get_children recursively from a figure and print an indented tree of types (depth-limited), with counts per level.
import collections
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([0, 1], [0, 1], label="line")
ax.scatter([0.5], [0.5], label="pt")
ax.legend()
ax.set_title("tree")
fig.colorbar(ax.imshow(np.eye(2), extent=(0, 1, 0, 1)), ax=ax)
levels = collections.Counter()
lines = []


def walk(artist, depth=0):
    levels[depth] += 1
    if depth <= 3:
        lines.append("  " * depth + type(artist).__name__)
    for child in artist.get_children():
        walk(child, depth + 1)


walk(fig)
print("\n".join(lines[:40]))
print(sorted(levels.items()))
print(len(lines))
seen = set()
dupes = 0
def ids(a):
    global dupes
    if id(a) in seen:
        dupes += 1
        return
    seen.add(id(a))
    for c in a.get_children():
        ids(c)
ids(fig)
print(len(seen), dupes)
