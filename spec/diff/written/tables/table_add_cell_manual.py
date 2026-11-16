# Build a table cell by cell with Table and add_cell, including merged-looking header cells.
import matplotlib.pyplot as plt
from matplotlib.table import Table

fig, ax = plt.subplots(figsize=(5, 2.5))
ax.set_axis_off()
tb = Table(ax, bbox=[0, 0, 1, 1])
w, h = 1 / 4, 1 / 4
for j, txt in enumerate(["", "Mon", "Tue", "Wed"]):
    tb.add_cell(0, j, w, h, text=txt, loc="center", facecolor="lightgray")
for i, name in enumerate(["morning", "noon", "evening"], start=1):
    tb.add_cell(i, 0, w, h, text=name, loc="left")
    for j in range(1, 4):
        tb.add_cell(i, j, w, h, text=str(i * j), loc="center")
ax.add_table(tb)
fig.savefig("manual_table.png")
print(len(tb.get_celld()))
