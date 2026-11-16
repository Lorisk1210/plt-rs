# CI pipeline: stage durations as a Gantt per run, success/failure markers, and a duration trend with rolling median.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(201)
stages = ["checkout", "build", "test", "deploy"]
runs = 15
fig, (a, b) = plt.subplots(1, 2, figsize=(13, 5), gridspec_kw={"width_ratios": [1.5, 1]})
totals, ok = [], []
for r in range(runs):
    start = 0
    for k, s in enumerate(stages):
        d = rng.gamma(2 + k, 20)
        a.broken_barh([(start, d)], (r - 0.4, 0.8), color=f"C{k}")
        start += d
    success = rng.random() > 0.2
    a.plot(start + 10, r, "o" if success else "x", color="g" if success else "r")
    totals.append(start); ok.append(success)
a.set_yticks(range(runs), [f"#{1000 + r}" for r in range(runs)])
a.set_xlabel("seconds")
b.plot(totals, "o-", color="0.5")
b.plot(np.convolve(totals, np.ones(5) / 5, mode="valid"), color="C0", lw=2)
b.scatter(np.flatnonzero(~np.array(ok)), np.array(totals)[~np.array(ok)], color="r", zorder=3)
print([round(t, 1) for t in totals[:3]], int(np.sum(ok)))
fig.savefig("ci.png", dpi=50)
