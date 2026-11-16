# Table with cell background colours by value (a coloured grid), edge colours and bold cells.
import numpy as np
import matplotlib.pyplot as plt

vals = np.array([[0.91, 0.42, 0.13], [0.55, 0.77, 0.30], [0.12, 0.66, 0.98]])
cmap = plt.cm.RdYlGn
fig, ax = plt.subplots(figsize=(4.5, 2.5))
ax.set_axis_off()
tbl = ax.table(cellText=[[f"{v:.2f}" for v in row] for row in vals], cellColours=cmap(vals),
               rowLabels=["model A", "model B", "model C"], colLabels=["recall", "precision", "f1"],
               loc="center", edges="horizontal")
for (r, c), cell in tbl.get_celld().items():
    if r > 0 and c >= 0 and vals[r - 1, c] > 0.9:
        cell.get_text().set_fontweight("bold")
tbl.scale(1, 1.6)
fig.savefig("scores.png")
print(sorted(tbl.get_celld())[:4])
