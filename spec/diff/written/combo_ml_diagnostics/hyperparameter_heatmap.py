# Grid search results as an annotated heat map with the best cell outlined and a line plot of the best row.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

Cs = [0.01, 0.1, 1, 10, 100]
gammas = [1e-4, 1e-3, 1e-2, 1e-1]
score = np.array([[0.5 + 0.4 * np.exp(-((np.log10(c) - 1) ** 2 + (np.log10(g) + 2) ** 2) / 3) for c in Cs] for g in gammas])
bi, bj = np.unravel_index(score.argmax(), score.shape)
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4))
im = a.imshow(score, cmap="viridis")
for (i, j), v in np.ndenumerate(score):
    a.text(j, i, f"{v:.2f}", ha="center", va="center", color="w" if v < 0.75 else "k", fontsize=8)
a.add_patch(Rectangle((bj - 0.5, bi - 0.5), 1, 1, fill=False, ec="red", lw=2))
a.set_xticks(range(5), [str(c) for c in Cs])
a.set_yticks(range(4), [f"{g:g}" for g in gammas])
a.set_xlabel("C")
a.set_ylabel("gamma")
fig.colorbar(im, ax=a)
b.semilogx(Cs, score[bi], "o-")
b.set_title(f"gamma = {gammas[bi]:g}")
print((int(bi), int(bj)), round(float(score.max()), 4))
fig.savefig("grid_search.png", dpi=50)
