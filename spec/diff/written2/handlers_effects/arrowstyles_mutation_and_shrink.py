# FancyArrowPatch with every ArrowStyle, mutation_scale and shrinkA/B, reading back the computed path extents per style.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyArrowPatch, ArrowStyle

fig, ax = plt.subplots(figsize=(5, 6))
ax.set_xlim(0, 4)
ax.set_ylim(0, len(ArrowStyle.get_styles()) + 1)
names = sorted(ArrowStyle.get_styles())
arrows = []
for k, name in enumerate(names, start=1):
    a = FancyArrowPatch((0.5, k), (3.5, k), arrowstyle=name, mutation_scale=15, shrinkA=5, shrinkB=10, color=f"C{k % 10}")
    ax.add_patch(a)
    ax.text(0.05, k, name, va="center", fontsize=7)
    arrows.append(a)
fig.canvas.draw()
for name, a in zip(names, arrows):
    ext = a.get_path().get_extents()
    print(name, [round(v, 3) for v in ext.bounds])
custom = ArrowStyle("Fancy", head_length=0.8, head_width=0.6, tail_width=0.3)
print(type(custom).__name__, custom.head_length, custom.head_width)
a0 = arrows[0]
a0.set_arrowstyle("->", head_length=1.2)
a0.set_mutation_scale(25)
print(type(a0.get_arrowstyle()).__name__, a0.get_mutation_scale(), a0.get_arrowstyle().head_length)
fig.savefig("arrowstyles.png")
a1 = arrows[1]
a1.set_positions((1, 1.5), (3, 2.5))
a1.set_connectionstyle("arc3", rad=0.4)
fig.canvas.draw()
print([round(v, 3) for v in a1.get_path().get_extents().bounds], type(a1.get_connectionstyle()).__name__)
