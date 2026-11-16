# Football season: league position over matchdays (inverted y), goals for/against bars, result pie and a shot map.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle

rng = np.random.default_rng(161)
md = np.arange(1, 35)
pos = np.clip(np.round(8 - np.cumsum(rng.normal(0.15, 0.8, 34)) * 0.4), 1, 18)
gf, ga = rng.poisson(1.6, 34), rng.poisson(1.1, 34)
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 2, 1)
a.plot(md, pos, "o-")
a.invert_yaxis()
a.set_ylabel("position")
b = fig.add_subplot(2, 2, 2)
b.bar(md, gf, color="C2", label="for")
b.bar(md, -ga, color="C3", label="against")
b.legend()
c = fig.add_subplot(2, 2, 3)
res = [(gf > ga).sum(), (gf == ga).sum(), (gf < ga).sum()]
c.pie(res, labels=["W", "D", "L"], colors=["C2", "0.7", "C3"], autopct="%d%%")
d = fig.add_subplot(2, 2, 4)
d.add_patch(Rectangle((0, 0), 105, 68, fill=False))
d.add_patch(Circle((52.5, 34), 9.15, fill=False))
d.plot([52.5, 52.5], [0, 68], "k", lw=0.8)
shots = rng.uniform([60, 10], [104, 58], (40, 2))
goal = rng.random(40) < 0.15
d.scatter(shots[:, 0], shots[:, 1], c=np.where(goal, "C2", "C0"), s=np.where(goal, 60, 20))
d.set_aspect("equal"); d.axis("off")
fig.tight_layout()
print(res, int(pos[-1]), int(gf.sum()), int(ga.sum()))
fig.savefig("season.png", dpi=50)
