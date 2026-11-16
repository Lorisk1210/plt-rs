# A table with long strings: users shrink font, widen columns and set cell heights.
import matplotlib.pyplot as plt

notes = [["2024-01", "Initial release with the core features"],
         ["2024-03", "Faster import; fixed crash on empty files"],
         ["2024-07", "New export formats, better accessibility of colours"]]
fig, ax = plt.subplots(figsize=(7, 2))
ax.axis("off")
tbl = ax.table(cellText=notes, colLabels=["date", "change"], colWidths=[0.15, 0.8], cellLoc="left", loc="center")
tbl.auto_set_font_size(False)
tbl.set_fontsize(8)
for (r, c), cell in tbl.get_celld().items():
    cell.set_height(0.2)
fig.savefig("changes.png", dpi=100)
print([cell.get_text().get_text() for (r, c), cell in sorted(tbl.get_celld().items()) if r == 0])
