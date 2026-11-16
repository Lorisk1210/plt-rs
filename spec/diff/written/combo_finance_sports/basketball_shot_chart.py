# Shot chart: court lines with patches, hexbin of shot locations coloured by make rate (C with reduce), and zone summary bars.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Arc, Rectangle, Circle

rng = np.random.default_rng(162)
x = rng.normal(0, 8, 600); y = np.abs(rng.normal(8, 7, 600))
dist = np.hypot(x, y)
made = (rng.random(600) < np.clip(0.65 - dist / 50, 0.1, 0.7)).astype(float)
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 5), gridspec_kw={"width_ratios": [1.6, 1]})
hb = a.hexbin(x, y, C=made, reduce_C_function=np.mean, gridsize=18, cmap="RdYlGn", extent=(-25, 25, 0, 35), mincnt=3)
a.add_patch(Circle((0, 0), 0.75, fill=False))
a.add_patch(Rectangle((-8, -1), 16, 19, fill=False))
a.add_patch(Arc((0, 0), 47.5, 47.5, theta1=22, theta2=158))
a.set_xlim(-25, 25); a.set_ylim(-2, 35); a.set_aspect("equal")
fig.colorbar(hb, ax=a, label="FG%")
zones = [("paint", dist < 8), ("mid", (dist >= 8) & (dist < 23.75)), ("three", dist >= 23.75)]
b.bar([z for z, _ in zones], [made[m].mean() for _, m in zones], color="C0")
b.set_ylim(0, 1)
print([(z, int(m.sum()), round(float(made[m].mean()), 3)) for z, m in zones])
fig.savefig("shots.png", dpi=50)
