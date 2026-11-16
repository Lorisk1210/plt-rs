# Quarterly figures with a PeriodIndex as bar labels and a year-over-year growth line.
import pandas as pd
import matplotlib.pyplot as plt

q = pd.period_range("2022Q1", "2024Q4", freq="Q")
revenue = pd.Series([10, 12, 11, 15, 13, 15, 14, 19, 16, 18, 17, 23], index=q)
growth = revenue.pct_change(4) * 100
fig, ax = plt.subplots(figsize=(8, 4))
ax.bar(range(len(q)), revenue.values, tick_label=[str(p) for p in q])
ax2 = ax.twinx()
ax2.plot(range(len(q)), growth.values, "ko-")
ax2.set_ylabel("YoY growth %")
plt.setp(ax.get_xticklabels(), rotation=45)
print(growth.round(2).dropna().tolist())
fig.tight_layout()
fig.savefig("quarters.png")
