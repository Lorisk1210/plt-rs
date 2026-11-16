# HR dashboard: headcount by department (stacked), salary boxplots, tenure histogram, and a hires timeline.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
n = 300
df = pd.DataFrame({
    "dept": rng.choice(["eng", "sales", "ops", "hr"], n, p=[0.45, 0.25, 0.2, 0.1]),
    "level": rng.choice(["junior", "mid", "senior"], n, p=[0.4, 0.4, 0.2]),
    "salary": rng.normal(60, 12, n),
    "tenure": rng.exponential(3, n),
    "hired": pd.to_datetime("2018-01-01") + pd.to_timedelta(rng.integers(0, 2400, n), unit="D"),
})
df.loc[df.level == "senior", "salary"] += 25
fig, axs = plt.subplots(2, 2, figsize=(11, 8))
counts = pd.crosstab(df["dept"], df["level"])[["junior", "mid", "senior"]]
counts.plot(kind="bar", stacked=True, ax=axs[0, 0], rot=0, colormap="Pastel1", edgecolor="k")
axs[0, 0].set_title("headcount")
groups = [g["salary"].values for _, g in df.groupby("dept")]
axs[0, 1].boxplot(groups, tick_labels=sorted(df["dept"].unique()), showmeans=True)
axs[0, 1].set_ylabel("salary (k)")
axs[1, 0].hist(df["tenure"], bins=25, cumulative=True, density=True, histtype="step", lw=2)
axs[1, 0].set_xlabel("tenure (years)")
axs[1, 0].yaxis.set_major_formatter(plt.matplotlib.ticker.PercentFormatter(1))
hires = df.set_index("hired").resample("QE")["dept"].count()
axs[1, 1].step(hires.index, hires.cumsum(), where="post")
axs[1, 1].set_title("cumulative hires")
fig.autofmt_xdate()
fig.tight_layout()
print(counts.to_dict())
print([t.get_text() for t in axs[0, 1].get_xticklabels()])
fig.savefig("hr.png", dpi=60)
