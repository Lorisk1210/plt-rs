# Class results overview: grade distribution, scores per subject boxplot, top students table and correlation heat map.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
names = [f"S{i:02d}" for i in range(30)]
ability = rng.normal(0, 1, 30)
df = pd.DataFrame({sub: np.clip(70 + 10 * ability + rng.normal(0, 8, 30), 0, 100).round() for sub in ["math", "physics", "art", "history"]}, index=names)
df["mean"] = df.mean(axis=1)
grades = pd.cut(df["mean"], [0, 50, 60, 70, 80, 101], labels=["F", "D", "C", "B", "A"], right=False)
fig, axs = plt.subplots(2, 2, figsize=(11, 8))
gc = grades.value_counts().reindex(["A", "B", "C", "D", "F"])
axs[0, 0].bar(gc.index, gc.values, color=plt.cm.RdYlGn(np.linspace(0.9, 0.1, 5)))
axs[0, 0].set_title("grades")
axs[0, 1].boxplot(df[["math", "physics", "art", "history"]].values, tick_labels=["math", "physics", "art", "history"])
top = df.sort_values("mean", ascending=False).head(5)[["math", "physics", "mean"]].round(1)
axs[1, 0].axis("off")
axs[1, 0].table(cellText=top.values, rowLabels=top.index, colLabels=top.columns, loc="center")
axs[1, 0].set_title("top five")
corr = df[["math", "physics", "art", "history"]].corr()
im = axs[1, 1].imshow(corr, vmin=-1, vmax=1, cmap="coolwarm")
axs[1, 1].set_xticks(range(4), corr.columns, rotation=45)
axs[1, 1].set_yticks(range(4), corr.columns)
fig.colorbar(im, ax=axs[1, 1])
fig.tight_layout()
print(gc.to_dict(), top.index.tolist())
fig.savefig("class.png", dpi=60)
