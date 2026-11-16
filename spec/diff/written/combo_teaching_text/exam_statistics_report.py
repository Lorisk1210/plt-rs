# Exam statistics: score histogram with grade boundaries, per-question difficulty bars, and a discrimination scatter.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(172)
ability = rng.normal(0, 1, 150)
difficulty = np.linspace(-1.5, 1.5, 12)
answers = (ability[:, None] - difficulty[None, :] + rng.normal(0, 1, (150, 12))) > 0
scores = answers.sum(axis=1)
fig, axs = plt.subplots(1, 3, figsize=(14, 4))
axs[0].hist(scores, bins=np.arange(-0.5, 13.5), color="0.6", edgecolor="w")
for cut, grade in [(4, "D"), (6, "C"), (8, "B"), (10, "A")]:
    axs[0].axvline(cut - 0.5, color="C3", ls="--")
    axs[0].text(cut, axs[0].get_ylim()[1] * 0.9, grade, color="C3")
p = answers.mean(axis=0)
axs[1].bar(range(1, 13), p, color=plt.cm.RdYlGn(p))
axs[1].set_ylim(0, 1)
disc = np.array([np.corrcoef(answers[:, q], scores)[0, 1] for q in range(12)])
axs[2].scatter(p, disc, c=range(12), cmap="viridis")
for q in range(12):
    axs[2].annotate(str(q + 1), (p[q], disc[q]), fontsize=7)
fig.tight_layout()
print(np.round(p, 3).tolist()[:4], round(float(scores.mean()), 3))
fig.savefig("exam.png", dpi=50)
