# Window extents of a legend's parts (frame, texts, handles) and a colorbar's axes, outline and tick labels, across locations.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4), dpi=80)
for k in range(3):
    ax.plot([0, 1], [k, k + 1], label=f"series {k}")
im = ax.imshow(np.linspace(0, 1, 16).reshape(4, 4), extent=(0, 1, 0, 3), aspect="auto")
cb = fig.colorbar(im, ax=ax, shrink=0.8)
r = fig.canvas.get_renderer()
for loc in ("upper left", "center", "lower right", "best"):
    leg = ax.legend(loc=loc, title="legend")
    fig.canvas.draw()
    print(loc, [round(v, 1) for v in leg.get_window_extent(r).bounds],
          [round(v, 1) for v in leg.get_frame().get_window_extent(r).bounds])
    print([round(t.get_window_extent(r).x0, 1) for t in leg.get_texts()],
          [round(h.get_window_extent(r).width, 1) for h in leg.legend_handles])
print([round(v, 1) for v in leg.get_title().get_window_extent(r).bounds])
print([round(v, 1) for v in cb.ax.get_window_extent(r).bounds], [round(v, 1) for v in cb.outline.get_window_extent(r).bounds])
print([round(t.get_window_extent(r).y0, 1) for t in cb.ax.get_yticklabels()])
print([round(v, 1) for v in cb.ax.get_tightbbox(r).bounds])
leg2 = fig.legend(loc="outside upper center", ncols=3)
fig.canvas.draw()
print([round(v, 1) for v in leg2.get_window_extent(r).bounds])
fig.savefig("legend_cb.png")
cb.set_label("colour scale")
fig.canvas.draw()
print([round(v, 1) for v in cb.ax.yaxis.label.get_window_extent(r).bounds], [round(v, 1) for v in cb.ax.get_tightbbox(r).bounds])
