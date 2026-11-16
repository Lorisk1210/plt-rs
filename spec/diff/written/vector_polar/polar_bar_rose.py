# Wind rose: polar bars of wind direction frequency, 16 sectors, compass labels.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
directions = np.concatenate([rng.normal(250, 30, 400), rng.normal(90, 20, 150)]) % 360
counts, edges = np.histogram(directions, bins=16, range=(0, 360))
theta = np.deg2rad(edges[:-1] + 11.25)
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="polar")
bars = ax.bar(theta, counts, width=np.deg2rad(22.5), bottom=0, color=plt.cm.viridis(counts / counts.max()), edgecolor="k")
ax.set_theta_zero_location("N")
ax.set_theta_direction(-1)
ax.set_xticks(np.deg2rad(np.arange(0, 360, 45)), ["N", "NE", "E", "SE", "S", "SW", "W", "NW"])
print(counts.tolist())
fig.savefig("rose.png")
