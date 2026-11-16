# Measure text and axes extents in pixels with the renderer to size a label box.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3), dpi=100)
t = ax.text(0.5, 0.5, "Measure me", fontsize=20, ha="center")
fig.canvas.draw()
r = fig.canvas.get_renderer()
bb = t.get_window_extent(renderer=r)
print(round(bb.width), round(bb.height))
tight = ax.get_tightbbox(r)
print([round(v) for v in tight.bounds])
w, h, d = r.get_text_width_height_descent("Measure me", t.get_fontproperties(), ismath=False)
print(round(w), round(h), round(d))
fig.savefig("measured.png")
