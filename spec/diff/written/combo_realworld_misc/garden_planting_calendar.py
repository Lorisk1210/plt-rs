# Garden plan: sowing/harvest windows per crop as broken_barh with month ticks, a bed layout grid and a yield bar chart.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

crops = {"tomato": [(3, 2), (7, 3)], "lettuce": [(2, 3), (5, 2)], "carrot": [(3, 2), (7, 2)], "beans": [(4, 2), (7, 2)]}
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 1, 1)
for k, (crop, (sow, harvest)) in enumerate(crops.items()):
    a.broken_barh([sow], (k - 0.35, 0.7), color="C2")
    a.broken_barh([harvest], (k - 0.35, 0.7), color="C1")
a.set_yticks(range(4), list(crops)); a.set_xticks(range(1, 13), list("JFMAMJJASOND")); a.set_xlim(1, 13)
b = fig.add_subplot(2, 2, 3)
for i in range(2):
    for j in range(4):
        b.add_patch(Rectangle((j * 1.1, i * 1.1), 1, 1, color=plt.cm.Pastel1((i * 4 + j) % 9)))
        b.text(j * 1.1 + 0.5, i * 1.1 + 0.5, list(crops)[(i * 4 + j) % 4], ha="center", va="center", fontsize=8)
b.set_xlim(-0.1, 4.5); b.set_ylim(-0.1, 2.3); b.set_aspect("equal"); b.axis("off")
c = fig.add_subplot(2, 2, 4)
c.bar(list(crops), [12, 6, 8, 5], color="C1")
fig.tight_layout()
print(len(a.collections), len(b.patches))
fig.savefig("garden.png", dpi=50)
