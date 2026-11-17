# Quiver colour handling: C arrays, color= lists, cmap and norm, with masked U/V; colours after drawing and masked arrow counts.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import colors

X, Y = np.meshgrid(np.arange(5), np.arange(3))
U = np.ma.masked_array(np.ones((3, 5)), mask=np.zeros((3, 5), bool))
U.mask[1, 2] = True
V = np.linspace(-1, 1, 15).reshape(3, 5)
fig, axs = plt.subplots(1, 3, figsize=(8, 3), dpi=50)
q1 = axs[0].quiver(X, Y, U, V, np.hypot(U, V), cmap="viridis", norm=colors.Normalize(0, 2))
q2 = axs[1].quiver(X, Y, U, V, color=["r", "g", "b"])
q3 = axs[2].quiver(X.ravel(), Y.ravel(), U.ravel(), V.ravel(), color="k", alpha=0.5)
fig.canvas.draw()
print(q1.get_array().shape, np.ma.count_masked(q1.U), np.round(q1.get_facecolor()[:2], 3).tolist())
print(np.round(q2.get_facecolor()[:4], 3).tolist())
print(np.round(q3.get_facecolor(), 3).tolist(), q3.get_alpha())
print(q1.N, q2.N, len(q1.get_offsets()), q1.get_offsets()[:2].tolist())
q1.set_UVC(U * -1, V)
fig.canvas.draw()
print(np.round(q1.get_array()[:3], 3).tolist())
fig.savefig("quiver_colors.png")
q2.set_color("m")
q2.set_alpha(0.3)
fig.canvas.draw()
print(np.round(q2.get_facecolor(), 3).tolist(), q2.get_alpha())
q3.set_offsets(np.c_[np.arange(15) * 0.5, np.zeros(15)])
fig.canvas.draw()
print(q3.get_offsets()[:2].tolist(), [round(v, 2) for v in axs[2].get_xlim()])
