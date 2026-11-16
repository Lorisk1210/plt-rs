# Delivery routes: depots and stops scatter, route polylines with arrows (annotate), distances per route bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(211)
depot = np.array([5, 5])
routes = {f"van {k}": np.vstack([depot, rng.uniform(0, 10, (5, 2)), depot]) for k in range(3)}
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 5), gridspec_kw={"width_ratios": [1.3, 1]})
dists = {}
for k, (name, pts) in enumerate(routes.items()):
    line, = a.plot(pts[:, 0], pts[:, 1], "-o", ms=4, label=name)
    for p0, p1 in zip(pts[:-1], pts[1:]):
        a.annotate("", xy=(p0 + p1) / 2 + (p1 - p0) * 0.05, xytext=(p0 + p1) / 2, arrowprops=dict(arrowstyle="->", color=line.get_color()))
    dists[name] = float(np.hypot(*np.diff(pts, axis=0).T).sum())
a.plot(*depot, "ks", ms=12, label="depot")
a.set_aspect("equal"); a.legend()
b.bar(list(dists), list(dists.values()), color=[l.get_color() for l in a.lines[:3]])
b.set_ylabel("km")
print({k: round(v, 3) for k, v in dists.items()})
fig.savefig("routes.png", dpi=50)
