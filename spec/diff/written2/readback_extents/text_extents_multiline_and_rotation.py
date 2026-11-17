# Text window extents and get_tightbbox for multiline, rotated, wrapped and bbox-framed texts; linespacing changes.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4), dpi=100)
specs = [
    dict(s="one line"), dict(s="two\nlines"), dict(s="three\nlines\nhere", linespacing=2.0),
    dict(s="rotated 45", rotation=45), dict(s="rotated 90\nmulti", rotation=90, ha="center"),
    dict(s="framed", bbox=dict(boxstyle="round,pad=0.5", fc="w")),
    dict(s="a long sentence that should wrap at the axes edge when wrapping is enabled", wrap=True),
    dict(s="anchor rot", rotation=30, rotation_mode="anchor", ha="right"),
]
texts = [ax.text(0.1, 0.9 - k * 0.11, **spec) for k, spec in enumerate(specs)]
fig.canvas.draw()
r = fig.canvas.get_renderer()
for t in texts:
    w = t.get_window_extent(r)
    tb = t.get_tightbbox(r)
    print(repr(t.get_text()[:12]), [round(v, 1) for v in w.bounds], [round(v, 1) for v in tb.bounds])
framed = texts[5]
print([round(v, 1) for v in framed.get_bbox_patch().get_window_extent(r).bounds])
texts[1].set_linespacing(3)
fig.canvas.draw()
print([round(v, 1) for v in texts[1].get_window_extent(r).bounds])
print(texts[6]._get_wrapped_text().count("\n"), texts[6].get_wrap())
fig.savefig("text_extents.png")
texts[3].set_rotation(-45)
fig.canvas.draw()
print([round(v, 1) for v in texts[3].get_window_extent(r).bounds], texts[3].get_rotation())
print([round(v, 1) for v in texts[0].get_window_extent(dpi=200).bounds])
