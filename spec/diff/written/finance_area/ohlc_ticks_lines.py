# OHLC bars drawn with LineCollection (vertical high-low plus open/close ticks).
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

rng = np.random.default_rng(12)
n = 25
o = 20 + rng.normal(0, 1, n).cumsum()
c = o + rng.normal(0, 0.8, n)
h = np.maximum(o, c) + rng.uniform(0.1, 1, n)
l = np.minimum(o, c) - rng.uniform(0.1, 1, n)
segs = []
for i in range(n):
    segs += [[(i, l[i]), (i, h[i])], [(i - 0.3, o[i]), (i, o[i])], [(i, c[i]), (i + 0.3, c[i])]]
colors = [("g" if c[i] >= o[i] else "r") for i in range(n) for _ in range(3)]
fig, ax = plt.subplots(figsize=(8, 4))
ax.add_collection(LineCollection(segs, colors=colors, linewidths=1.5))
ax.autoscale()
ax.set_title("OHLC")
print(len(segs))
fig.savefig("ohlc.png")
