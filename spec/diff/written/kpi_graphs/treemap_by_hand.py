# Slice-and-dice treemap of budget shares with rectangles sized by value and labels inside.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

items = [("housing", 35), ("food", 20), ("transport", 15), ("health", 10), ("leisure", 12), ("other", 8)]
total = sum(v for _, v in items)
fig, ax = plt.subplots(figsize=(7, 4))
x = 0.0
for k, (name, v) in enumerate(items):
    w = v / total
    ax.add_patch(Rectangle((x, 0), w, 1, facecolor=plt.cm.Set3(k), edgecolor="white", lw=2))
    ax.text(x + w / 2, 0.5, f"{name}\n{v}%", ha="center", va="center", rotation=90 if w < 0.12 else 0, fontsize=9)
    x += w
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
ax.axis("off")
print(round(x, 6), len(ax.patches))
fig.savefig("treemap.png")
