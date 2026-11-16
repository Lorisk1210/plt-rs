# Personal finance: monthly income vs expenses bars, savings rate line, category treemap-like bars, and net worth area.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(234)
months = pd.period_range("2024-01", periods=12, freq="M")
income = np.full(12, 3200.0) + rng.normal(0, 50, 12)
expenses = 2400 + rng.normal(0, 200, 12)
fig, axs = plt.subplots(2, 2, figsize=(12, 7))
x = np.arange(12)
axs[0, 0].bar(x - 0.2, income, 0.4, label="income"); axs[0, 0].bar(x + 0.2, expenses, 0.4, label="expenses")
axs[0, 0].set_xticks(x, months.strftime("%b")); axs[0, 0].legend()
rate = (income - expenses) / income
axs[0, 1].plot(x, rate * 100, "o-"); axs[0, 1].axhline(20, ls="--", color="k")
cats = {"rent": 1100, "food": 450, "transport": 180, "fun": 250, "other": 300}
left = 0
for k, (c, v) in enumerate(cats.items()):
    axs[1, 0].barh(0, v, left=left, color=f"C{k}"); axs[1, 0].text(left + v / 2, 0, c, ha="center", va="center", fontsize=8)
    left += v
axs[1, 0].axis("off")
net = 10000 + np.cumsum(income - expenses)
axs[1, 1].fill_between(x, net, color="C2", alpha=0.4); axs[1, 1].plot(x, net, "C2")
fig.tight_layout()
print(round(float(rate.mean()), 4), round(float(net[-1]), 2))
fig.savefig("finance.png", dpi=50)
