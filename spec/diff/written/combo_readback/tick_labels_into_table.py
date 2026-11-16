# After drawing, read tick label texts and positions and write them into a table under the plot.
import numpy as np
import matplotlib.pyplot as plt

fig, (ax, tab) = plt.subplots(2, 1, figsize=(6, 5), gridspec_kw={"height_ratios": [3, 1]})
ax.plot(np.linspace(0, 1e4, 50), np.sqrt(np.linspace(0, 1e4, 50)))
fig.canvas.draw()
labels = [t.get_text() for t in ax.get_xticklabels()]
positions = [round(float(t.get_position()[0]), 1) for t in ax.get_xticklabels()]
print(labels, positions)
tab.axis("off")
tab.table(cellText=[labels, [str(p) for p in positions]], rowLabels=["label", "x"], loc="center")
print(ax.xaxis.get_major_formatter().format_data(2500.0), ax.xaxis.get_major_formatter().format_data_short(2500.0))
fig.savefig("tick_table.png", dpi=50)
