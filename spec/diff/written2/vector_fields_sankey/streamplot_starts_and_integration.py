# streamplot with start_points, broken_streamlines off, integration_direction, density and maxlength; line and arrow counts.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

y, x = np.mgrid[-2:2:40j, -3:3:60j]
u = -1 - x ** 2 + y
v = 1 + x - y ** 2
fig, axs = plt.subplots(2, 2, figsize=(7, 6), dpi=50)
variants = [dict(density=0.6), dict(start_points=[[0, 0], [1, 1], [-2, -1]], density=1),
            dict(broken_streamlines=False, density=0.4), dict(integration_direction="forward", maxlength=1.0, density=0.8)]
for ax, kw in zip(axs.flat, variants):
    sp = ax.streamplot(x, y, u, v, **kw)
    segs = sp.lines.get_segments()
    print(sorted(kw), type(sp).__name__, len(segs) > 0, type(sp.arrows).__name__)
    print(sorted(kw), round(float(np.concatenate(segs)[:, 0].min()), 3) >= -3.01, round(float(np.concatenate(segs)[:, 1].max()), 3) <= 2.01)
speed = np.hypot(u, v)
sp = axs[0, 0].streamplot(x, y, u, v, color=speed, linewidth=speed / speed.max() * 2, cmap="autumn", density=0.3,
                          arrowstyle="->", arrowsize=1.5)
fig.canvas.draw()
print(sp.lines.get_array() is not None, len(np.atleast_1d(sp.lines.get_linewidth())) > 1)
try:
    axs[0, 1].streamplot(x, y, u, v, start_points=[[10, 10]])
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.savefig("streams.png")
masked_u = np.ma.masked_where(np.hypot(x, y) < 0.5, u)
sp2 = axs[1, 1].streamplot(x, y, masked_u, v, density=0.5, color="k")
fig.canvas.draw()
print(len(sp2.lines.get_segments()) > 0, np.round(sp2.lines.get_color()[0], 3).tolist())
