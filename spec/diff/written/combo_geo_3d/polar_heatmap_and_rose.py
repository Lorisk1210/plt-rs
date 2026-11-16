# Polar heat map (pcolormesh on polar axes) of wind speed by direction and hour, plus a rose of totals.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(61)
theta = np.linspace(0, 2 * np.pi, 17)
r = np.arange(0, 25)
speed = np.abs(np.sin(theta[:-1, None] * 2 + r[None, :-1] / 4)) * 10 + rng.random((16, 24))
fig = plt.figure(figsize=(10, 4.5))
a = fig.add_subplot(1, 2, 1, projection="polar")
mesh = a.pcolormesh(theta, r, speed.T, cmap="viridis", shading="flat")
a.set_theta_zero_location("N")
a.set_theta_direction(-1)
fig.colorbar(mesh, ax=a, pad=0.1, label="m/s")
b = fig.add_subplot(1, 2, 2, projection="polar")
b.bar(theta[:-1] + np.pi / 16, speed.sum(axis=1), width=2 * np.pi / 16, color=plt.cm.viridis(speed.sum(axis=1) / speed.sum(axis=1).max()))
b.set_theta_zero_location("N")
b.set_theta_direction(-1)
print(mesh.get_array().shape, round(float(speed.sum()), 3))
fig.savefig("polar_heat.png", dpi=50)
