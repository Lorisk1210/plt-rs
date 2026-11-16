# Questionnaire scales: correlation heat map, the strongest pair as a scatter with jitter and fit, and scale means.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(74)
latent = rng.normal(size=200)
df = pd.DataFrame({f"Q{i}": np.clip(np.round(3 + latent * w + rng.normal(0, 1, 200)), 1, 5) for i, w in enumerate([0.9, 0.7, 0.2, -0.5, 0.8], start=1)})
corr = df.corr().copy()
corr = corr.where(~np.eye(5, dtype=bool))
i, j = np.unravel_index(np.nanargmax(np.abs(corr.values)), corr.shape)
fig, axs = plt.subplots(1, 3, figsize=(13, 4))
im = axs[0].imshow(corr, cmap="RdBu_r", vmin=-1, vmax=1)
axs[0].set_xticks(range(5), corr.columns)
axs[0].set_yticks(range(5), corr.columns)
fig.colorbar(im, ax=axs[0])
xi, yj = df.iloc[:, i], df.iloc[:, j]
axs[1].scatter(xi + rng.uniform(-0.2, 0.2, 200), yj + rng.uniform(-0.2, 0.2, 200), s=8, alpha=0.5)
coef = np.polyfit(xi, yj, 1)
axs[1].plot([1, 5], np.polyval(coef, [1, 5]), "r")
axs[1].set(xlabel=corr.columns[i], ylabel=corr.columns[j])
axs[2].bar(df.columns, df.mean(), yerr=df.std() / np.sqrt(200), capsize=3)
fig.tight_layout()
print(corr.columns[i], corr.columns[j], round(float(corr.iloc[i, j]), 4))
fig.savefig("questionnaire.png", dpi=50)
