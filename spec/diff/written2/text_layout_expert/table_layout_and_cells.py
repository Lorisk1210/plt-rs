# Table layout: auto_set_font_size, auto column widths, scale, cell edges ('open', 'horizontal', 'BRTL'), and cell extents read back.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, ax = plt.subplots(figsize=(5, 3), dpi=80)
ax.set_axis_off()
cells = [["1", "two", "three hundred"], ["4.5", "x", "a much longer cell text"], ["7", "8", "9"]]
tab = ax.table(cellText=cells, colLabels=["A", "B", "C"], rowLabels=["r1", "r2", "r3"], loc="center",
               cellLoc="right", colLoc="center", edges="closed")
fig.canvas.draw()
r = fig.canvas.get_renderer()
print(sorted(tab.get_celld())[:5], len(tab.get_celld()))
print([round(tab[0, c].get_width(), 4) for c in range(3)], round(tab[1, 0].get_height(), 4))
tab.auto_set_font_size(False)
tab.set_fontsize(7)
tab.auto_set_column_width([0, 1, 2])
tab.scale(1.2, 1.5)
fig.canvas.draw()
print([round(tab[1, c].get_width(), 4) for c in range(3)], round(tab[1, 0].get_height(), 4), tab[1, 2].get_text().get_fontsize())
print([round(v, 1) for v in tab.get_window_extent(r).bounds])
print([round(v, 1) for v in tab[2, 2].get_window_extent(r).bounds], tab[2, 2].get_text().get_text())
for edges in ("open", "horizontal", "vertical", "BR", "closed"):
    tab[1, 1].visible_edges = edges
    print(edges, len(tab[1, 1].get_path().vertices), tab[1, 1].visible_edges)
tab[0, 0].set_facecolor("lightblue")
tab[0, 0].set_text_props(weight="bold", color="navy")
print(tab[0, 0].get_text().get_weight(), tab[0, 0].get_facecolor())
cell = tab.add_cell(4, 1, 0.2, 0.1, text="added", loc="left")
fig.canvas.draw()
print(cell.get_text().get_text(), (4, 1) in tab.get_celld(), tab.contains(MouseEvent("motion_notify_event", fig.canvas, 200, 120))[0])
fig.savefig("table.png")
