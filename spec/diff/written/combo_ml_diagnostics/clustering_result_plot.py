# k-means by hand on 2-D blobs: points coloured by cluster, centroids, decision regions with contourf and an elbow plot.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(43)
X = np.vstack([rng.normal(c, 0.4, (60, 2)) for c in [(0, 0), (3, 1), (1, 3)]])
def kmeans(X, k, iters=20):
    c = X[:k].copy()
    for _ in range(iters):
        lab = np.argmin(((X[:, None] - c[None]) ** 2).sum(-1), axis=1)
        c = np.array([X[lab == j].mean(axis=0) for j in range(k)])
    return c, lab, ((X - c[lab]) ** 2).sum()
centers, labels, _ = kmeans(X, 3)
xx, yy = np.meshgrid(np.linspace(-1.5, 4.5, 200), np.linspace(-1.5, 4.5, 200))
grid_lab = np.argmin(((np.c_[xx.ravel(), yy.ravel()][:, None] - centers[None]) ** 2).sum(-1), axis=1).reshape(xx.shape)
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4.5))
a.contourf(xx, yy, grid_lab, levels=[-0.5, 0.5, 1.5, 2.5], cmap="Pastel1", alpha=0.6)
sc = a.scatter(X[:, 0], X[:, 1], c=labels, cmap="Set1", s=12)
a.scatter(*centers.T, marker="X", s=200, c="k")
a.legend(*sc.legend_elements(), title="cluster", loc="upper right")
inertia = [kmeans(X, k)[2] for k in range(1, 7)]
b.plot(range(1, 7), inertia, "o-")
b.set_xlabel("k")
b.set_ylabel("inertia")
print(np.round(centers, 3).tolist(), [round(float(v), 2) for v in inertia])
fig.savefig("kmeans.png", dpi=50)
