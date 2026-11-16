# Wind field as arrows with quiver, coloured by speed, plus a quiverkey for scale.
import numpy as np
import matplotlib.pyplot as plt

x, y = np.meshgrid(np.linspace(-2, 2, 15), np.linspace(-2, 2, 15))
u = -y
v = x
speed = np.hypot(u, v)
fig, ax = plt.subplots(figsize=(5.5, 5))
q = ax.quiver(x, y, u, v, speed, cmap="viridis", scale=30, width=0.004)
ax.quiverkey(q, X=0.85, Y=1.03, U=2, label="2 m/s", labelpos="E")
ax.set_aspect("equal")
fig.colorbar(q, ax=ax, label="speed")
print(q.N, round(float(speed.max()), 4))
fig.savefig("vortex.png")
