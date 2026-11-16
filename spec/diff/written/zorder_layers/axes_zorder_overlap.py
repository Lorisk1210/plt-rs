# Overlapping axes: which one is on top (add order vs set_zorder), with opaque backgrounds.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 4))
a = fig.add_axes([0.1, 0.1, 0.6, 0.6], facecolor="lightblue")
b = fig.add_axes([0.35, 0.35, 0.6, 0.6], facecolor="mistyrose")
a.plot([0, 1], [1, 0])
b.plot([0, 1], [0, 1])
fig.savefig("b_on_top.png")
a.set_zorder(1)
fig.savefig("a_on_top.png")
print(a.get_zorder(), b.get_zorder(), [ax.get_zorder() for ax in fig.axes])
fig.savefig("axes_zorder_overlap_small.png", dpi=40)
print(fig.get_size_inches(), len(fig.axes))
