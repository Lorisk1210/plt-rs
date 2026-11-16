# Place a table with an explicit bbox in axes coordinates and auto-size its columns.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot([1, 2, 3, 4], [10, 20, 15, 30], "o-")
cells = [["Q1", "10"], ["Q2", "20"], ["Q3", "15"], ["Q4", "30"]]
tbl = ax.table(cellText=cells, colLabels=["quarter", "value"], bbox=[0.62, 0.05, 0.35, 0.4])
tbl.auto_set_font_size(False)
tbl.set_fontsize(8)
tbl.auto_set_column_width([0, 1])
ax.set_title("plot with embedded table")
fig.savefig("embedded.png")
print(len(tbl.get_celld()), tbl.get_celld()[(0, 0)].get_text().get_text())
