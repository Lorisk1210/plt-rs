# Portfolio report: allocation donut, cumulative returns per asset with end labels, a correlation heat map and a risk/return scatter.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(160)
assets = ["stocks", "bonds", "gold", "reits", "cash"]
weights = np.array([0.45, 0.3, 0.1, 0.1, 0.05])
rets = pd.DataFrame(rng.multivariate_normal([5e-4, 2e-4, 3e-4, 4e-4, 1e-4], np.diag([1e-4, 2e-5, 6e-5, 1.2e-4, 1e-7]), 500), columns=assets)
fig, axs = plt.subplots(2, 2, figsize=(11, 8))
wedges, _, _ = axs[0, 0].pie(weights, labels=assets, autopct="%d%%", wedgeprops=dict(width=0.4))
cum = (1 + rets).cumprod()
for asset, w in zip(assets, wedges):
    axs[0, 1].plot(cum[asset], color=w.get_facecolor())
    axs[0, 1].text(len(cum), cum[asset].iloc[-1], asset, color=w.get_facecolor(), fontsize=8)
im = axs[1, 0].imshow(rets.corr(), vmin=-1, vmax=1, cmap="coolwarm")
axs[1, 0].set_xticks(range(5), assets, rotation=45); axs[1, 0].set_yticks(range(5), assets)
fig.colorbar(im, ax=axs[1, 0])
vol, mean = rets.std() * np.sqrt(252), rets.mean() * 252
axs[1, 1].scatter(vol, mean, s=weights * 2000, c=[w.get_facecolor() for w in wedges])
for a, x, y in zip(assets, vol, mean):
    axs[1, 1].annotate(a, (x, y), xytext=(5, 5), textcoords="offset points")
fig.tight_layout()
print(cum.iloc[-1].round(4).to_dict())
fig.savefig("portfolio.png", dpi=50)
