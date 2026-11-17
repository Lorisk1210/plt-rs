# Figure and axes get_tightbbox with artists excluded via set_in_layout, bbox_extra_artists, and for_layout_only.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
ax.plot([0, 1], [0, 1])
ax.set_title("title")
ax.set_xlabel("x label")
far = ax.text(1.6, 0.5, "far right", transform=ax.transAxes)
low = ax.annotate("below", (0.5, 0), (0.5, -0.6), textcoords="axes fraction", xycoords="axes fraction")
fig.canvas.draw()
r = fig.canvas.get_renderer()


def b(bb):
    return [round(v, 1) for v in bb.bounds]


print("axes", b(ax.get_tightbbox(r)))
print("axes layout", b(ax.get_tightbbox(r, for_layout_only=True)))
print("figure", b(fig.get_tightbbox(r)))
far.set_in_layout(False)
print("far out", b(ax.get_tightbbox(r)), b(fig.get_tightbbox(r)))
print("extra", b(fig.get_tightbbox(r, bbox_extra_artists=[far])))
low.set_in_layout(False)
print("both out", b(ax.get_tightbbox(r)))
print("only axes", b(ax.get_tightbbox(r, bbox_extra_artists=[])))
ax.xaxis.set_in_layout(False)
print("no xaxis", b(ax.get_tightbbox(r)))
ax.title.set_visible(False)
print("no title", b(ax.get_tightbbox(r)), b(ax.get_tightbbox(r, call_axes_locator=False)))
print(far.get_in_layout(), ax.xaxis.get_in_layout(), ax.get_in_layout())
fig.savefig("tight.png", bbox_inches="tight")
from PIL import Image
with Image.open("tight.png") as im:
    print(im.size)
