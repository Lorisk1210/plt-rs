# NPS breakdown: stacked single bar of detractors/passives/promoters, score histogram coloured by category, and word counts.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(72)
scores = rng.choice(np.arange(11), 500, p=np.r_[np.full(7, 0.04), [0.08, 0.15, 0.25, 0.24]])
det, pas, pro = (scores <= 6).mean(), ((scores >= 7) & (scores <= 8)).mean(), (scores >= 9).mean()
words = {"price": 45, "support": 38, "speed": 30, "bugs": 22, "docs": 15, "ui": 12}
fig, axs = plt.subplots(3, 1, figsize=(8, 8), gridspec_kw={"height_ratios": [1, 2, 2]})
left = 0
for share, color, name in [(det, "#d7191c", "detractors"), (pas, "#ffffbf", "passives"), (pro, "#1a9641", "promoters")]:
    axs[0].barh(0, share, left=left, color=color, edgecolor="k")
    axs[0].text(left + share / 2, 0, f"{name}\n{share:.0%}", ha="center", va="center", fontsize=8)
    left += share
axs[0].set_xlim(0, 1)
axs[0].axis("off")
axs[0].set_title(f"NPS = {100 * (pro - det):.0f}")
counts = np.bincount(scores, minlength=11)
axs[1].bar(range(11), counts, color=["#d7191c"] * 7 + ["#ffffbf"] * 2 + ["#1a9641"] * 2, edgecolor="k")
axs[1].set_xticks(range(11))
axs[2].barh(list(words), list(words.values()), color="slategray")
axs[2].invert_yaxis()
fig.tight_layout()
print(round(float(100 * (pro - det)), 2), counts.tolist())
fig.savefig("nps.png", dpi=50)
