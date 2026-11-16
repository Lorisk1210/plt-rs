# Read each bar's x, width, height (get_x/get_width/get_height/get_xy) to place custom labels and brackets.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
bars = ax.bar(["ctrl", "A", "B"], [4.2, 5.8, 7.1], yerr=[0.3, 0.4, 0.5], capsize=4, color="0.75")
geo = [(round(b.get_x(), 3), round(b.get_width(), 3), round(b.get_height(), 3), tuple(round(v, 3) for v in b.get_xy())) for b in bars]
print(geo)
def bracket(i, j, y, text):
    xi = bars[i].get_x() + bars[i].get_width() / 2
    xj = bars[j].get_x() + bars[j].get_width() / 2
    ax.plot([xi, xi, xj, xj], [y, y + 0.2, y + 0.2, y], color="k", lw=1)
    ax.text((xi + xj) / 2, y + 0.25, text, ha="center")
bracket(0, 1, 6.5, "*")
bracket(0, 2, 7.9, "**")
ax.set_ylim(0, 9)
print(round(bars[2].get_bbox().y1, 3), bars[1].get_center().tolist())
fig.savefig("brackets.png", dpi=50)
