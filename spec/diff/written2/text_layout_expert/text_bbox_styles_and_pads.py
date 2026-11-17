# Text bbox with every BoxStyle, pad values and mutation aspect; the patch extents relative to the text extent.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import BoxStyle

fig = plt.figure(figsize=(5, 6), dpi=80)
styles = sorted(BoxStyle.get_styles())
r = fig.canvas.get_renderer()
texts = []
for k, name in enumerate(styles):
    t = fig.text(0.2, 0.95 - k * 0.09, f"{name} box", bbox=dict(boxstyle=name, fc="w"), fontsize=10)
    texts.append((name, t))
fig.canvas.draw()
for name, t in texts:
    tb = t.get_window_extent(r)
    pb = t.get_bbox_patch().get_window_extent(r)
    print(name, round(pb.width - tb.width, 1), round(pb.height - tb.height, 1))
t = texts[0][1]
for pad in (0, 0.5, 1.5):
    t.set_bbox(dict(boxstyle=f"round,pad={pad}", fc="y"))
    fig.canvas.draw()
    print(pad, [round(v, 1) for v in t.get_bbox_patch().get_window_extent(r).bounds])
t.get_bbox_patch().set_mutation_aspect(2)
fig.canvas.draw()
print([round(v, 1) for v in t.get_bbox_patch().get_window_extent(r).bounds])
t.set_backgroundcolor("pink")
print(t.get_bbox_patch().get_facecolor())
fig.savefig("boxes.png")
t.set_bbox(None)
fig.canvas.draw()
print(t.get_bbox_patch())
