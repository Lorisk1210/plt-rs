# Quiver with angles/scale_units/scale/units combinations; arrow polygon sizes read after drawing, and quiverkey placement.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

X, Y = np.meshgrid(np.arange(4), np.arange(3))
U, V = np.ones_like(X, float), np.linspace(0, 1, 12).reshape(3, 4)
fig, axs = plt.subplots(2, 3, figsize=(9, 5), dpi=50)
specs = [dict(), dict(angles="xy", scale_units="xy", scale=1), dict(units="inches", scale=2),
         dict(units="dots", width=3), dict(pivot="mid", headwidth=6, headlength=8), dict(minlength=3, minshaft=2)]
for ax, kw in zip(axs.flat, specs):
    q = ax.quiver(X, Y, U, V, **kw)
    ax.set_xlim(-1, 5)
    ax.set_ylim(-1, 3)
    fig.canvas.draw()
    verts = q.get_paths()[0].vertices if q.get_paths() else np.zeros((0, 2))
    print(sorted(kw), q.N, round(float(q.scale), 4) if q.scale else None, q.units, q.scale_units, len(verts),
          np.round(np.ptp(verts, axis=0), 4).tolist() if len(verts) else None)
q = axs[0, 0].quiver(X, Y, U, V)
qk = axs[0, 0].quiverkey(q, 0.8, 1.05, 1, "1 m/s", labelpos="E", coordinates="axes")
fig.canvas.draw()
print(type(qk).__name__, qk.U, qk.label, [round(v, 1) for v in qk.get_window_extent(fig.canvas.get_renderer()).bounds][:2])
q.set_UVC(U * 2, V * 2, C=np.arange(12))
fig.canvas.draw()
print(q.get_array().tolist()[:4], round(float(q.scale), 4))
fig.savefig("quiver.png")
qk.set_visible(False)
print(qk.get_visible(), q.width, q.headwidth, q.headlength, q.headaxislength, q.pivot)
q.set_offsets(np.c_[X.ravel() + 0.5, Y.ravel()])
fig.canvas.draw()
print(q.get_offsets()[:2].tolist(), q.XY[:2].tolist())
