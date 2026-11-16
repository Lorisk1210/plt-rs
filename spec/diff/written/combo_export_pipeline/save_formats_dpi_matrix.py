# Save one figure at several dpi values and bbox options; read PNG sizes back and tabulate them in a second figure.
import itertools
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([0, 1, 2], [1, 0, 2])
ax.set_title("sizes")
rows = []
for dpi, bbox in itertools.product([50, 100], [None, "tight"]):
    name = f"s_{dpi}_{bbox}.png"
    fig.savefig(name, dpi=dpi, bbox_inches=bbox)
    with Image.open(name) as im:
        rows.append([str(dpi), str(bbox), f"{im.size[0]}x{im.size[1]}"])
t_fig, t_ax = plt.subplots(figsize=(4, 2))
t_ax.axis("off")
t_ax.table(cellText=rows, colLabels=["dpi", "bbox", "size"], loc="center")
t_fig.savefig("size_table.png", dpi=60)
print(rows)
