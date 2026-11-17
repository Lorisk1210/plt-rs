# Histogram of 10^6 values into 1000 bins with histtype step and bar, and a 2000-bar bar chart; counts, edges and patch numbers.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

n = 1_000_000
vals = np.sin(np.arange(n) * 0.618033988749) * np.sqrt(np.arange(n) % 1000 / 1000.0)
fig, axs = plt.subplots(1, 3, figsize=(9, 3), dpi=60)
c1, e1, p1 = axs[0].hist(vals, bins=1000, histtype="bar")
c2, e2, p2 = axs[1].hist(vals, bins=1000, histtype="step", density=True)
bars = axs[2].bar(np.arange(2000), np.sqrt(np.arange(2000)), width=1.0)
fig.canvas.draw()
print(int(c1.sum()), int(c1.max()), round(float(e1[0]), 5), round(float(e1[-1]), 5), len(p1))
print(round(float(c2.max()), 4), type(p2[0]).__name__, len(p2[0].get_xy()))
print(len(bars), round(float(bars[-1].get_height()), 4), [round(v, 2) for v in axs[2].get_xlim()])
fig2, ax2 = plt.subplots(figsize=(4, 4), dpi=60)
h2, xe, ye, im = ax2.hist2d(vals[::10], np.cos(np.arange(n // 10)), bins=200)
print(h2.shape, int(h2.sum()), int(h2.max()), type(im).__name__)
fig.savefig("hist_big.png")
fig2.savefig("hist2d_big.png")
c3, e3, p3 = axs[1].hist([vals[:n // 2], vals[n // 2:]], bins=50, histtype="barstacked")
print(c3.shape, int(c3[-1].sum()), len(p3), len(p3[0]))
c4, e4, p4 = axs[1].hist(vals, bins=np.linspace(-1, 1, 11), cumulative=True, histtype="stepfilled")
print(c4.astype(int).tolist()[-3:], type(p4[0]).__name__)
w = np.where(vals > 0, 2.0, 1.0)
c5, _, _ = axs[0].hist(vals, bins=4, weights=w)
print(c5.tolist())
counts, edges = np.histogram(vals, bins=1000)
print(bool((counts == c1.astype(int)).all()), bool(np.allclose(edges, e1)))
