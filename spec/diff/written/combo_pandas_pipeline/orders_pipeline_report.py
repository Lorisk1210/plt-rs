# Orders pipeline: clean -> groupby -> pivot -> plot (pandas line + bar), then a summary table from the same frame.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(110)
raw = pd.DataFrame({"ts": pd.date_range("2024-01-01", periods=1000, freq="7h"),
                    "category": rng.choice(["books", "games", "music", None], 1000, p=[0.4, 0.3, 0.25, 0.05]),
                    "amount": rng.gamma(3, 12, 1000)})
clean = raw.dropna(subset=["category"]).assign(month=lambda d: d["ts"].dt.to_period("M"))
monthly = clean.pivot_table(index="month", columns="category", values="amount", aggfunc="sum")
fig, (a, b, c) = plt.subplots(3, 1, figsize=(9, 10), gridspec_kw={"height_ratios": [2, 2, 1]})
monthly.plot(ax=a, marker="o", title="monthly revenue")
share = monthly.div(monthly.sum(axis=1), axis=0)
share.plot(kind="bar", stacked=True, ax=b, rot=0, legend=False, color=[l.get_color() for l in a.get_lines()])
b.set_xticklabels([str(p) for p in share.index])
c.axis("off")
summary = clean.groupby("category")["amount"].agg(["count", "sum", "mean"]).round(1)
c.table(cellText=summary.values, rowLabels=summary.index, colLabels=summary.columns, loc="center")
fig.tight_layout()
print(len(raw) - len(clean), summary.to_dict())
print([l.get_label() for l in a.get_lines()], [p.get_facecolor()[:3] for p in b.patches][::len(monthly)][:3])
fig.savefig("orders.png", dpi=50)
