# Lotka-Volterra: populations over time on twin axes, phase portrait coloured by time, and a quiver field of the dynamics.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

a, b, c, d, dt = 1.0, 0.1, 1.5, 0.075, 0.01
x, y = [10.0], [5.0]
for _ in range(3000):
    x.append(x[-1] + dt * (a * x[-1] - b * x[-1] * y[-1]))
    y.append(y[-1] + dt * (-c * y[-1] + d * x[-1] * y[-1]))
x, y = np.array(x), np.array(y)
t = np.arange(x.size) * dt
fig, (p, q) = plt.subplots(1, 2, figsize=(12, 4.5))
p.plot(t, x, "C0"); tw = p.twinx(); tw.plot(t, y, "C3")
p.set_ylabel("prey", color="C0"); tw.set_ylabel("predators", color="C3")
segs = np.stack([np.c_[x[:-1], y[:-1]], np.c_[x[1:], y[1:]]], axis=1)
lc = LineCollection(segs, array=t[:-1], cmap="viridis")
q.add_collection(lc); q.autoscale()
X, Y = np.meshgrid(np.linspace(5, 40, 12), np.linspace(2, 25, 12))
q.quiver(X, Y, a * X - b * X * Y, -c * Y + d * X * Y, color="0.6", angles="xy")
fig.colorbar(lc, ax=q, label="time")
print(round(float(x.max()), 3), round(float(y.max()), 3))
fig.savefig("predator_prey.png", dpi=50)
