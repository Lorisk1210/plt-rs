# Annotation arrows passing under scatter points: arrow zorder vs collection zorder.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4))
ax.scatter([1, 2, 3], [1, 2, 3], s=400, color="gold", zorder=3, edgecolors="k")
ann = ax.annotate("through", xy=(3, 3), xytext=(0.6, 3.2), arrowprops=dict(arrowstyle="->", lw=2), zorder=2)
ax.annotate("over", xy=(2, 2), xytext=(3.2, 1.0), arrowprops=dict(arrowstyle="->", lw=2), zorder=4)
ax.set_xlim(0, 4)
ax.set_ylim(0, 4)
print(ann.get_zorder(), ax.texts[1].get_zorder())
fig.savefig("arrows.png")
fig.savefig("annotation_arrow_behind_points_small.png", dpi=40)
print(fig.get_size_inches(), len(fig.axes))
