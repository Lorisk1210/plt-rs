# Subfigures with their own layout settings: constrained parent with subfigures, colorbars and legends inside, suptitles at each level.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(7, 4), dpi=60, layout="constrained")
top, bottom = fig.subfigures(2, 1, height_ratios=[1, 2])
top.suptitle("top part")
axt = top.subplots(1, 3)
for k, a in enumerate(axt):
    a.plot([0, 1], [0, k], label=f"l{k}")
top.legend(loc="outside right upper")
left, right = bottom.subfigures(1, 2, wspace=0.05)
axl = left.subplots(2, 1, sharex=True)
im = axl[0].imshow(np.eye(3))
left.colorbar(im, ax=axl, shrink=0.8)
axr = right.add_subplot(projection="polar")
right.suptitle("polar")
right.set_facecolor("0.95")
fig.suptitle("root")
fig.canvas.draw()
for a in list(axt) + list(axl) + [axr]:
    print([round(v, 3) for v in a.get_position().bounds])
print([round(v, 3) for v in top.bbox_relative.bounds], [round(v, 3) for v in right.bbox_relative.bounds])
print(len(fig.axes), len(fig.subfigs), len(bottom.subfigs), [len(sf.legends) for sf in (top, bottom)])
fig.savefig("subfigs_layout.png")
print([round(v, 1) for v in fig.get_tightbbox().bounds])
right.set_visible(False)
fig.canvas.draw()
print([round(v, 3) for v in axl[0].get_position().bounds], axr.get_visible(), right.get_visible())
