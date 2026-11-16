# Hatch colour separate from edge colour (hatchcolor) and hatch line width per artist.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 3.5))
b1 = ax.bar([0, 1, 2], [3, 4, 2], color="white", edgecolor="black", hatch="//", hatchcolor="red")
b2 = ax.bar([3, 4], [2, 5], color="lightyellow", edgecolor="navy", hatch="xx")
for bar in b2:
    bar.set_hatch_linewidth(2.5)
print(b1[0].get_hatchcolor(), b2[0].get_hatch_linewidth())
fig.savefig("hatch_colors.png")
fig.savefig("hatch_colors.svg", metadata={"Date": None})
fig.savefig("hatch_color_linewidth_small.png", dpi=40)
print(fig.get_size_inches().tolist())
