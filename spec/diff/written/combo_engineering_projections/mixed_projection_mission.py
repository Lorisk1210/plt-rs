# Mission overview: 3-D trajectory, a polar plot of heading, a Mollweide ground track and a cartesian altitude profile.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 4 * np.pi, 400)
x, y, z = np.cos(t) * (1 + t / 10), np.sin(t) * (1 + t / 10), t / 4
fig = plt.figure(figsize=(12, 8))
a = fig.add_subplot(2, 2, 1, projection="3d"); a.plot(x, y, z); a.set_title("trajectory")
p = fig.add_subplot(2, 2, 2, projection="polar"); p.plot(np.arctan2(np.gradient(y), np.gradient(x)), np.linspace(0.2, 1, 400)); p.set_title("heading")
m = fig.add_subplot(2, 2, 3, projection="mollweide")
lon = (t * 0.8) % (2 * np.pi) - np.pi
m.plot(lon, 0.6 * np.sin(t), ".", ms=2); m.grid(True); m.set_title("ground track")
c = fig.add_subplot(2, 2, 4); c.plot(t, z); c.set(xlabel="t", ylabel="altitude"); c.set_title("profile")
fig.tight_layout()
print([ax.name for ax in fig.axes], [round(v, 3) for v in a.get_zlim()])
fig.savefig("mission.png", dpi=50)
