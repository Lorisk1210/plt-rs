# Build a table, read every cell's text back, and restyle cells whose numbers exceed a threshold.
import numpy as np
import matplotlib.pyplot as plt

vals = np.array([[12, 45, 7], [33, 8, 51], [5, 27, 40]])
fig, ax = plt.subplots(figsize=(4, 2.5))
ax.axis("off")
tbl = ax.table(cellText=vals.astype(str), colLabels=["a", "b", "c"], loc="center")
hot = []
for (r, c), cell in tbl.get_celld().items():
    txt = cell.get_text().get_text()
    if r > 0 and int(txt) > 30:
        cell.set_facecolor("#ffcccc")
        cell.get_text().set_fontweight("bold")
        hot.append((r, c, txt))
print(sorted(hot))
fig.savefig("table_restyle.png", dpi=50)
