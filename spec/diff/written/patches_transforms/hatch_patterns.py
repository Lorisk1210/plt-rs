# Table of hatch patterns and densities on rectangles, with hatch colour and linewidth via rcParams.
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

hatches = ["/", "\\", "|", "-", "+", "x", "o", "O", ".", "*", "//", "xx", "oo", "/o", "\\|", "+."]
plt.rcParams["hatch.linewidth"] = 1.5
fig, ax = plt.subplots(figsize=(8, 4))
for i, h in enumerate(hatches):
    x, y = i % 8, i // 8
    ax.add_patch(Rectangle((x, y * 1.5), 0.8, 1.0, fill=False, hatch=h, edgecolor="C%d" % (i % 10)))
    ax.text(x + 0.4, y * 1.5 - 0.2, repr(h), ha="center", fontsize=8)
ax.set_xlim(-0.2, 8)
ax.set_ylim(-0.5, 2.7)
ax.axis("off")
fig.savefig("hatches.png")
print(len(ax.patches))
