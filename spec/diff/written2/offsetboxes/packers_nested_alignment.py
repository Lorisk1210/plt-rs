# Nest HPacker and VPacker with every align and mode, read back each child's offset and the packed extents.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import HPacker, VPacker, TextArea, DrawingArea, AnchoredOffsetbox
from matplotlib.patches import Rectangle


def swatch(w, h, color):
    da = DrawingArea(w, h)
    da.add_artist(Rectangle((0, 0), w, h, fc=color))
    return da


fig, ax = plt.subplots(figsize=(6, 5))
renderer = fig.canvas.get_renderer()
for align in ("baseline", "bottom", "top", "left", "right", "center"):
    for mode in ("fixed", "expand", "equal"):
        row = HPacker(children=[swatch(20, 10, "C0"), TextArea("text"), swatch(10, 30, "C1")],
                      align=align, mode=mode, pad=2, sep=4, width=120)
        row.set_figure(fig)
        bbox = row.get_bbox(renderer)
        print(align, mode, [round(v, 2) for v in bbox.bounds])
col = VPacker(children=[TextArea("title", textprops=dict(weight="bold")),
                        HPacker(children=[swatch(15, 15, "C2"), TextArea("a")], sep=3),
                        HPacker(children=[swatch(15, 15, "C3"), TextArea("longer label")], sep=3)],
              align="left", pad=4, sep=2)
col.set_figure(fig)
box = AnchoredOffsetbox(loc="upper right", child=col, frameon=True, borderpad=0.5)
ax.add_artist(box)
fig.canvas.draw()
print([round(v, 2) for v in col.get_bbox(renderer).bounds])
print([round(v, 2) for v in box.get_window_extent(renderer).bounds])
print([type(c).__name__ for c in col.get_children()], len(box.get_children()))
fig.savefig("packers.png")
