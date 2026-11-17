# Annotation arrows with connectionstyles (arc3, angle3, angle, arc, bar), shrink and patchA/B; path extents for each.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 5), dpi=80)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
styles = ["arc3,rad=0.3", "arc3,rad=-0.3", "angle3,angleA=90,angleB=0", "angle,angleA=-90,angleB=180,rad=5",
          "arc,angleA=-90,angleB=0,armA=30,armB=30,rad=5", "bar,fraction=0.3", "bar,angle=180,fraction=-0.2"]
anns = []
for k, cs in enumerate(styles):
    a = ax.annotate(f"c{k}", xy=(2 + k, 2), xytext=(1 + k, 7 - k * 0.5), arrowprops=dict(arrowstyle="-|>",
                    connectionstyle=cs, shrinkA=3, shrinkB=6), bbox=dict(boxstyle="round", fc="w"))
    anns.append(a)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for cs, a in zip(styles, anns):
    p = a.arrow_patch.get_path()
    print(cs.split(",")[0], len(p.vertices), [round(v, 1) for v in p.get_extents().bounds])
a = anns[0]
print(type(a.arrow_patch).__name__, a.arrow_patch.patchA is a.get_bbox_patch(), a.arrow_patch.shrinkA, a.arrow_patch.shrinkB)
a.arrow_patch.set_connectionstyle("arc3", rad=0.0)
fig.canvas.draw()
print(len(a.arrow_patch.get_path().vertices))
a.arrow_patch.set_patchA(None)
fig.canvas.draw()
print([round(v, 1) for v in a.arrow_patch.get_path().get_extents().bounds])
print(sorted(matplotlib.patches.ConnectionStyle.get_styles()))
fig.savefig("connections.png")
