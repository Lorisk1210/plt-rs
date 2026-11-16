# Pie with many small slices grouped into "other", hatches and rotated labels.
import matplotlib.pyplot as plt

raw = {"A": 40, "B": 25, "C": 12, "D": 8, "E": 5, "F": 4, "G": 3, "H": 2, "I": 1}
big = {k: v for k, v in raw.items() if v >= 5}
big["other"] = sum(v for v in raw.values() if v < 5)
fig, ax = plt.subplots(figsize=(5, 5))
wedges, texts = ax.pie(big.values(), labels=big.keys(), rotatelabels=True, labeldistance=1.1,
                       hatch=["", "", "//", "", "", ".."], wedgeprops=dict(edgecolor="k"))
ax.set_title("Grouped small categories", pad=20)
print(big, [round(t.get_rotation(), 2) for t in texts])
fig.savefig("grouped_pie.png")
fig.savefig("pie_small_slices_hatch_80dpi.png", dpi=80)
print(fig.get_size_inches(), len(fig.axes))
plt.close(fig)
