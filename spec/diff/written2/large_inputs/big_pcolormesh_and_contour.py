# A 400x400 pcolormesh and contourf with 50 levels on it; path counts, levels and PNG/PDF sizes.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

n = 400
x = np.linspace(-3, 3, n)
X, Y = np.meshgrid(x, x)
Z = np.sin(X ** 2 + Y) * np.exp(-0.1 * (X ** 2 + Y ** 2))
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3), dpi=60)
qm = a1.pcolormesh(X, Y, Z, shading="auto", cmap="RdBu")
cs = a2.contourf(X, Y, Z, levels=50, cmap="RdBu")
fig.canvas.draw()
print(qm.get_array().shape, len(cs.levels), len(cs.get_paths()), sum(len(p.vertices) for p in cs.get_paths()) > 10_000)
png = io.BytesIO()
fig.savefig(png, format="png")
pdf = io.BytesIO()
fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
print(len(png.getvalue()) > 10_000, len(pdf.getvalue()) > 100_000)
qm.set_rasterized(True)
cs.set_rasterized(True)
pdf2 = io.BytesIO()
fig.savefig(pdf2, format="pdf", metadata={"CreationDate": None})
print(len(pdf2.getvalue()) < len(pdf.getvalue()))
print(np.round(cs.levels[[0, 25, -1]], 4).tolist())
qm.set_clim(-0.5, 0.5)
cs2 = a2.contour(X, Y, Z, levels=[-0.5, 0, 0.5], colors="k", linewidths=0.5)
fig.canvas.draw()
print(qm.get_clim(), len(cs2.get_paths()), [len(p.vertices) > 10 for p in cs2.get_paths()])
fig.savefig("big_mesh.png")
