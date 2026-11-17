# PaddedBox around multi-line TextAreas with multilinebaseline, a frame patch, and draw_frame toggled; extents of each piece.
import matplotlib
matplotlib.use("agg")
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.offsetbox import PaddedBox, TextArea, HPacker, AnchoredOffsetbox

fig = Figure(figsize=(5, 3), dpi=100)
FigureCanvasAgg(fig)
ax = fig.add_subplot()
r = fig.canvas.get_renderer()
texts = [TextArea("one line"), TextArea("two\nlines", multilinebaseline=True), TextArea("three\nshort\nlines")]
row = HPacker(children=texts, align="baseline", pad=0, sep=8)
padded = PaddedBox(row, pad=6, draw_frame=True, patch_attrs=dict(fc="lightyellow", ec="0.3"))
anch = AnchoredOffsetbox(loc="center", child=padded, frameon=False)
ax.add_artist(anch)
fig.canvas.draw()
for t in texts:
    print(repr(t.get_text()), t.get_multilinebaseline(), [round(v, 2) for v in t.get_bbox(r).bounds])
print([round(v, 2) for v in row.get_bbox(r).bounds])
print([round(v, 2) for v in padded.get_bbox(r).bounds])
print([round(v, 2) for v in anch.get_window_extent(r).bounds])
texts[0].set_text("changed\ntext")
texts[1].set_multilinebaseline(False)
fig.canvas.draw()
print([round(v, 2) for v in row.get_bbox(r).bounds], [round(v, 2) for v in padded.get_bbox(r).bounds])
padded.patch.set_visible(False)
padded.update_frame(padded.get_bbox(r), fontsize=12)
print(padded.patch.get_boxstyle().pad, [round(v, 2) for v in padded.patch.get_bbox().bounds])
fig.savefig("padded.png")
print(texts[2].get_text().count("\n"), len(row.get_children()))
