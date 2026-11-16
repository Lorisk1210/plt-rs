# Dump a subset of Axes.properties() and Figure.properties() keys and values, as users explore objects.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=80)
ax.plot([0, 1, 2], [1, 0, 1])
ax.set_title("props")
fig.canvas.draw()
ap = ax.properties()
fp = fig.properties()
for key in ["title", "xlim", "ylim", "xscale", "aspect", "facecolor", "frame_on", "navigate", "xlabel", "zorder", "visible", "adjustable"]:
    print("axes", key, repr(ap[key]))
for key in ["dpi", "size_inches", "facecolor", "edgecolor", "frameon", "figwidth", "tight_layout", "constrained_layout"]:
    print("figure", key, repr(fp[key]))
fig.savefig("props.png", dpi=40)
