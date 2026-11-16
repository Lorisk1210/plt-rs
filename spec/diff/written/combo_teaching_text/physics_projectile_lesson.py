# Projectile lesson: trajectories for several angles with apex markers, velocity vectors via quiver, and range vs angle.
import numpy as np
import matplotlib.pyplot as plt

g, v0 = 9.81, 20
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4.5))
ranges = []
for deg in (15, 30, 45, 60, 75):
    th = np.radians(deg)
    T = 2 * v0 * np.sin(th) / g
    t = np.linspace(0, T, 50)
    x, y = v0 * np.cos(th) * t, v0 * np.sin(th) * t - g * t ** 2 / 2
    line, = a.plot(x, y, label=f"{deg}°")
    a.plot(x[25], y[25], "^", color=line.get_color())
    a.quiver(x[::10], y[::10], np.full(5, v0 * np.cos(th)), v0 * np.sin(th) - g * t[::10], color=line.get_color(),
             angles="xy", scale_units="xy", scale=4, width=0.003)
    ranges.append(x[-1])
a.set_aspect("equal"); a.legend(); a.set_ylim(bottom=0)
angles = np.linspace(1, 89, 89)
b.plot(angles, v0 ** 2 * np.sin(2 * np.radians(angles)) / g)
b.axvline(45, ls=":", color="k")
b.set(xlabel="launch angle [°]", ylabel="range [m]")
print([round(float(r), 3) for r in ranges])
fig.savefig("projectile.png", dpi=50)
