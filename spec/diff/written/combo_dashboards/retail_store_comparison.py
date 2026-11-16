# Store comparison: grouped monthly bars, cumulative lines with end labels, margin scatter with size legend.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(8)
months = pd.period_range("2024-01", periods=12, freq="M")
stores = ["A", "B", "C"]
sales = pd.DataFrame(rng.integers(80, 160, (12, 3)), index=months, columns=stores)
fig, (a, b, c) = plt.subplots(3, 1, figsize=(9, 11))
x = np.arange(12)
for i, s in enumerate(stores):
    a.bar(x + (i - 1) * 0.27, sales[s], 0.27, label=s)
a.set_xticks(x, [m.strftime("%b") for m in months])
a.legend(ncols=3)
cum = sales.cumsum()
for s in stores:
    line, = b.plot(x, cum[s])
    b.text(11.2, cum[s].iloc[-1], f"{s}: {cum[s].iloc[-1]}", color=line.get_color(), va="center")
b.set_xlim(0, 13)
margin = rng.uniform(5, 25, 12)
sc = c.scatter(sales["A"], margin, s=sales["B"], c=x, cmap="viridis", alpha=0.7)
handles, labels = sc.legend_elements(prop="sizes", num=3, alpha=0.6)
c.legend(handles, labels, title="store B", loc="upper left")
c.set_xlabel("store A sales")
c.set_ylabel("margin %")
fig.tight_layout()
print(cum.iloc[-1].to_dict(), labels)
fig.savefig("stores.png", dpi=60)
