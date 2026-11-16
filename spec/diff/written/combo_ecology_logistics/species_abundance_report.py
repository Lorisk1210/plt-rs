# Ecology survey: rank-abundance curve (log y), diversity index bars per site, and a site x species presence matrix.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(210)
sites = ["meadow", "forest", "wetland", "urban"]
abund = {s: np.sort(rng.lognormal(2, 1 + k * 0.3, 25))[::-1] for k, s in enumerate(sites)}
def shannon(x):
    p = x / x.sum()
    return -(p * np.log(p)).sum()
fig, axs = plt.subplots(1, 3, figsize=(15, 4.5))
for s, v in abund.items():
    axs[0].semilogy(np.arange(1, 26), v, "o-", ms=3, label=s)
axs[0].legend(); axs[0].set_xlabel("rank")
H = [shannon(v) for v in abund.values()]
axs[1].bar(sites, H, color=[l.get_color() for l in axs[0].lines])
presence = rng.random((4, 25)) > 0.4
axs[2].imshow(presence, cmap="Greens", aspect="auto")
axs[2].set_yticks(range(4), sites)
fig.tight_layout()
print([round(float(h), 4) for h in H], presence.sum(axis=1).tolist())
fig.savefig("ecology.png", dpi=50)
