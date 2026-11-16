# Fantasy league: points per week heat map, cumulative standings bump chart with team colours, and weekly high-score bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(166)
teams = ["Owls", "Bears", "Sharks", "Hawks", "Wolves", "Foxes"]
pts = rng.normal(100, 15, (6, 12)).round()
cum = pts.cumsum(axis=1)
rank = (-cum).argsort(axis=0).argsort(axis=0) + 1
fig = plt.figure(figsize=(12, 7))
a = fig.add_subplot(2, 2, (1, 2))
im = a.imshow(pts, cmap="YlGn", aspect="auto")
a.set_yticks(range(6), teams)
fig.colorbar(im, ax=a)
b = fig.add_subplot(2, 2, 3)
for i, t in enumerate(teams):
    b.plot(range(1, 13), rank[i], "o-", label=t)
b.invert_yaxis()
b.legend(fontsize=7, ncols=2)
c = fig.add_subplot(2, 2, 4)
best = pts.argmax(axis=0)
c.bar(range(1, 13), pts.max(axis=0), color=[b.lines[i].get_color() for i in best])
fig.tight_layout()
print(rank[:, -1].tolist(), [teams[i] for i in best][:4])
fig.savefig("fantasy.png", dpi=50)
