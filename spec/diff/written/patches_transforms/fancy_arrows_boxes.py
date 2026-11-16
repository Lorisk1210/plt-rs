# Flow diagram from text boxes connected with FancyArrowPatch arrows of different styles.
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch

fig, ax = plt.subplots(figsize=(7, 3))
nodes = {"load": (0.1, 0.5), "clean": (0.4, 0.5), "model": (0.7, 0.7), "report": (0.7, 0.3), "ship": (0.95, 0.5)}
for name, (x, y) in nodes.items():
    ax.text(x, y, name, ha="center", va="center", bbox=dict(boxstyle="round", fc="lightyellow", ec="k"))
edges = [("load", "clean", "-|>"), ("clean", "model", "->"), ("clean", "report", "-[") , ("model", "ship", "simple"), ("report", "ship", "wedge")]
for a, b, style in edges:
    arrow = FancyArrowPatch(nodes[a], nodes[b], arrowstyle=style, mutation_scale=15, shrinkA=20, shrinkB=20,
                            connectionstyle="arc3,rad=0.1")
    ax.add_patch(arrow)
ax.set_xlim(0, 1.05)
ax.set_ylim(0.1, 0.9)
ax.axis("off")
print(len(ax.patches))
fig.savefig("flow.png")
