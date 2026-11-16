# A figure that is only a table (axis off), with header colours and column widths set.
import matplotlib.pyplot as plt

rows = [["Alice", "Engineering", "5", "92"], ["Bob", "Sales", "3", "78"], ["Chen", "Support", "7", "88"],
        ["Dara", "Engineering", "2", "95"]]
header = ["name", "team", "years", "score"]
fig, ax = plt.subplots(figsize=(6, 2))
ax.axis("off")
tbl = ax.table(cellText=rows, colLabels=header, loc="center", cellLoc="center", colWidths=[0.2, 0.35, 0.15, 0.15])
tbl.auto_set_font_size(False)
tbl.set_fontsize(10)
tbl.scale(1, 1.5)
for j in range(len(header)):
    tbl[0, j].set_facecolor("#40466e")
    tbl[0, j].get_text().set_color("white")
fig.savefig("team.png", bbox_inches="tight")
print(tbl[1, 0].get_text().get_text(), round(tbl[1, 0].get_height(), 4))
