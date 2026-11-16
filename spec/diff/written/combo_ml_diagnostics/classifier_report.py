# Classifier report with numpy only: confusion matrix heat map, ROC curve with AUC, and score histograms per class.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(40)
y = rng.integers(0, 2, 400)
score = np.clip(rng.normal(0.35 + 0.3 * y, 0.18), 0, 1)
pred = (score > 0.5).astype(int)
cm = np.array([[np.sum((y == i) & (pred == j)) for j in (0, 1)] for i in (0, 1)])
thr = np.linspace(1, 0, 101)
tpr = [(score[y == 1] >= t).mean() for t in thr]
fpr = [(score[y == 0] >= t).mean() for t in thr]
auc = np.trapezoid(tpr, fpr)
fig, axs = plt.subplots(1, 3, figsize=(12, 3.8))
im = axs[0].imshow(cm, cmap="Blues")
for (i, j), v in np.ndenumerate(cm):
    axs[0].text(j, i, v, ha="center", va="center", color="w" if v > cm.max() / 2 else "k")
axs[0].set_xticks([0, 1], ["pred 0", "pred 1"])
axs[0].set_yticks([0, 1], ["true 0", "true 1"])
axs[1].plot(fpr, tpr, label=f"AUC = {auc:.3f}")
axs[1].plot([0, 1], [0, 1], "k--", lw=0.8)
axs[1].set_aspect("equal")
axs[1].legend(loc="lower right")
axs[2].hist([score[y == 0], score[y == 1]], bins=20, label=["class 0", "class 1"], histtype="stepfilled", alpha=0.5)
axs[2].axvline(0.5, color="k", ls=":")
axs[2].legend()
fig.tight_layout()
print(cm.tolist(), round(float(auc), 4))
fig.savefig("classifier.png", dpi=50)
