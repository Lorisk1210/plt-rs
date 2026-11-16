# Sales by region and quarter: 3-D bars, the same data as an annotated heat map, and a table of totals.
import numpy as np
import matplotlib.pyplot as plt

data = np.array([[12, 15, 14, 18], [8, 9, 12, 11], [20, 18, 22, 25]])
regions, quarters = ["N", "S", "W"], ["Q1", "Q2", "Q3", "Q4"]
fig = plt.figure(figsize=(12, 4))
a = fig.add_subplot(1, 3, 1, projection="3d")
xx, yy = np.meshgrid(range(4), range(3))
a.bar3d(xx.ravel(), yy.ravel(), 0, 0.6, 0.6, data.ravel(), color=plt.cm.Blues(data.ravel() / data.max()))
a.set_xticks(np.arange(4) + 0.3, quarters)
a.set_yticks(np.arange(3) + 0.3, regions)
b = fig.add_subplot(1, 3, 2)
b.imshow(data, cmap="Blues")
for (i, j), v in np.ndenumerate(data):
    b.text(j, i, v, ha="center", va="center")
b.set_xticks(range(4), quarters)
b.set_yticks(range(3), regions)
c = fig.add_subplot(1, 3, 3)
c.axis("off")
c.table(cellText=[[r, str(t)] for r, t in zip(regions, data.sum(axis=1))] + [["total", str(data.sum())]], colLabels=["region", "sum"], loc="center")
print(data.sum(axis=1).tolist(), len(b.texts))
fig.savefig("sales3d.png", dpi=50)
