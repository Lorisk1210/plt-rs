# Accounts receivable aging: stacked bars per customer by age bucket, a total-by-bucket pie and a top-debtors table.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(165)
customers = [f"cust {c}" for c in "ABCDEFG"]
buckets = ["0-30", "31-60", "61-90", "90+"]
df = pd.DataFrame(rng.gamma(2, 1500, (7, 4)) * np.array([1, 0.6, 0.3, 0.2]), index=customers, columns=buckets).round(0)
fig = plt.figure(figsize=(12, 6))
a = fig.add_subplot(1, 2, 1)
df.plot(kind="barh", stacked=True, ax=a, colormap="RdYlGn_r")
a.invert_yaxis()
b = fig.add_subplot(2, 2, 2)
b.pie(df.sum(), labels=buckets, autopct="%d%%", colors=[p.get_facecolor() for p in a.patches[::7]])
c = fig.add_subplot(2, 2, 4)
c.axis("off")
top = df.sum(axis=1).sort_values(ascending=False).head(3)
c.table(cellText=[[k, f"{v:,.0f}"] for k, v in top.items()], colLabels=["customer", "open"], loc="center")
fig.tight_layout()
print(df.sum().to_dict(), top.index.tolist())
fig.savefig("aging.png", dpi=50)
