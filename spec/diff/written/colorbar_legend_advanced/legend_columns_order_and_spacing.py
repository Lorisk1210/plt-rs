# Legend with many entries: columnspacing, handlelength, title alignment and alignment of text.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(7, 4))
for k in range(9):
    ax.plot([0, 1], [k, k + 1], label=f"series {k}")
leg = ax.legend(ncols=3, columnspacing=0.8, handlelength=1.0, handleheight=1.5, title="all series",
                alignment="left", loc="upper left", fontsize=8)
fig.canvas.draw()
print(leg._alignment if hasattr(leg, "_alignment") else None, round(leg.get_window_extent().width, 1) > 0)
fig.savefig("legend_columns.png")
fig.savefig("legend_columns_order_and_spacing_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
