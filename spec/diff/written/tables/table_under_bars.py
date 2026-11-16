# Stacked bars with a data table underneath (the classic "bar chart with table" layout).
import numpy as np
import matplotlib.pyplot as plt

data = [[66386, 174296, 75131, 577908, 32015], [58230, 381139, 78045, 99308, 160454],
        [89135, 80552, 152558, 497981, 603535]]
columns = ("Freeze", "Wind", "Flood", "Quake", "Hail")
rows = ["100 year", "50 year", "10 year"]
colors = plt.cm.BuPu(np.linspace(0.3, 0.7, len(rows)))
fig, ax = plt.subplots(figsize=(7, 5))
offset = np.zeros(len(columns))
cell_text = []
for row, color in zip(data, colors):
    ax.bar(np.arange(len(columns)) + 0.3, row, 0.4, bottom=offset, color=color)
    offset = offset + row
    cell_text.append([f"{x / 1000:.1f}" for x in offset])
table = ax.table(cellText=cell_text, rowLabels=rows, rowColours=colors, colLabels=columns, loc="bottom")
fig.subplots_adjust(left=0.2, bottom=0.25)
ax.set_xticks([])
ax.set_ylabel("loss in $1000s")
print(len(table.get_celld()))
fig.savefig("losses.png")
