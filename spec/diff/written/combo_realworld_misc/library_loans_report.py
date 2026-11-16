# Library loans: loans per genre stacked by age group, overdue rate line, and a top-10 titles table.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(233)
genres = ["novel", "crime", "science", "kids", "history"]
ages = ["<18", "18-40", "40-65", "65+"]
loans = pd.DataFrame(rng.integers(20, 200, (5, 4)), index=genres, columns=ages)
overdue = pd.Series(rng.uniform(0.02, 0.15, 5), index=genres)
fig, (a, b) = plt.subplots(1, 2, figsize=(13, 4.5), gridspec_kw={"width_ratios": [1.6, 1]})
loans.plot(kind="bar", stacked=True, ax=a, rot=0, colormap="Set2")
tw = a.twinx(); tw.plot(range(5), overdue.values * 100, "ko-"); tw.set_ylabel("overdue %")
b.axis("off")
titles = [[f"Title {k}", genres[k % 5], str(int(n))] for k, n in enumerate(np.sort(rng.integers(10, 80, 10))[::-1])]
b.table(cellText=titles, colLabels=["title", "genre", "loans"], loc="center")
fig.tight_layout()
print(loans.sum().to_dict(), overdue.round(3).tolist())
fig.savefig("library.png", dpi=50)
