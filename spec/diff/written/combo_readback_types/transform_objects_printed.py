# Print transforms users inspect (types, is_affine, matrices of transData parts) while placing text with them.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 5)
fig.canvas.draw()
for name, tr in [("transData", ax.transData), ("transAxes", ax.transAxes), ("transFigure", fig.transFigure),
                 ("dpi_scale_trans", fig.dpi_scale_trans), ("xaxis_transform", ax.get_xaxis_transform())]:
    print(name, tr.is_affine, tr.input_dims, np.round(tr.transform([[1, 1]]), 3).tolist())
print(np.round(ax.transData.get_affine().get_matrix(), 4).tolist())
print(np.round(ax.transLimits.get_matrix(), 4).tolist())
ax.text(5, 0.5, "data", transform=ax.transData)
ax.text(0.5, 0.9, "axes", transform=ax.transAxes)
fig.savefig("transforms.png", dpi=50)
