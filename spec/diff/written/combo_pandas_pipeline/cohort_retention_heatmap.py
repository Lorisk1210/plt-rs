# Cohort retention: build cohorts by signup month, retention matrix via groupby, annotated heat map and retention curves.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(112)
users = pd.DataFrame({"user": range(600), "signup": pd.to_datetime("2024-01-01") + pd.to_timedelta(rng.integers(0, 180, 600), unit="D")})
events = users.loc[users.index.repeat(rng.integers(1, 8, 600))].copy()
events["active"] = events["signup"] + pd.to_timedelta(rng.exponential(40, len(events)).astype(int), unit="D")
events["cohort"] = events["signup"].dt.to_period("M")
events["age"] = (events["active"].dt.to_period("M") - events["cohort"]).apply(lambda d: d.n)
retention = events.groupby(["cohort", "age"])["user"].nunique().unstack(fill_value=0)
retention = retention.div(retention[0], axis=0).iloc[:, :6]
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4.5))
im = a.imshow(retention.values, cmap="Blues", vmin=0, vmax=1)
for (i, j), v in np.ndenumerate(retention.values):
    a.text(j, i, f"{v:.0%}", ha="center", va="center", fontsize=7, color="w" if v > 0.5 else "k")
a.set_yticks(range(len(retention)), [str(c) for c in retention.index])
a.set_xlabel("months since signup")
fig.colorbar(im, ax=a)
for cohort, row in retention.iterrows():
    b.plot(row.index, row.values, marker=".", label=str(cohort))
b.legend(fontsize=7)
print(retention.shape, retention.iloc[0].round(3).tolist())
fig.savefig("cohorts.png", dpi=50)
