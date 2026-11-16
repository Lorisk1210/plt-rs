# Polar scatter limited to a sector (thetamin/thetamax) with an inner hole (rorigin).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(15)
r = rng.uniform(1, 3, 100)
theta = rng.uniform(0, np.pi / 2, 100)
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="polar")
ax.scatter(theta, r, c=r, cmap="hsv", s=20 * r ** 2, alpha=0.7)
ax.set_thetamin(0)
ax.set_thetamax(90)
ax.set_rorigin(-1)
ax.set_rmax(3.2)
ax.set_rticks([1, 2, 3])
print(ax.get_thetamin(), ax.get_thetamax(), ax.get_rorigin())
fig.savefig("sector.png")
