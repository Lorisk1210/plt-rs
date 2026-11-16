# 2-D embedding (PCA by SVD) of grouped data with class colours, label-at-centroid texts and explained variance bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(45)
classes = ["cat", "dog", "fish", "bird"]
X = np.vstack([rng.normal(i, 1, (50, 8)) + rng.normal(0, 0.3, 8) * i for i in range(4)])
y = np.repeat(range(4), 50)
Xc = X - X.mean(axis=0)
U, S, Vt = np.linalg.svd(Xc, full_matrices=False)
Z = Xc @ Vt[:2].T
var = S ** 2 / (S ** 2).sum()
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4.5), gridspec_kw={"width_ratios": [2, 1]})
for k, name in enumerate(classes):
    pts = Z[y == k]
    sc = a.scatter(pts[:, 0], pts[:, 1], s=10, label=name, alpha=0.7)
    a.text(*pts.mean(axis=0), name, fontsize=11, fontweight="bold", color=sc.get_facecolor()[0], ha="center",
           bbox=dict(fc="w", alpha=0.7, ec="none"))
a.set_xlabel(f"PC1 ({var[0]:.0%})")
a.set_ylabel(f"PC2 ({var[1]:.0%})")
b.bar(range(1, 9), var, color="0.6")
b.plot(range(1, 9), np.cumsum(var), "ro-")
print(np.round(var[:3], 4).tolist(), [t.get_color() for t in a.texts][0].round(3).tolist())
fig.savefig("embedding.png", dpi=50)
