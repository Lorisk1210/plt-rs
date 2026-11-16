# Pie chart straight from a pandas value_counts() result, labels from the index.
import pandas as pd
import matplotlib.pyplot as plt

tickets = pd.Series(["bug", "feature", "bug", "question", "bug", "feature", "docs", "bug", "question", "bug"])
counts = tickets.value_counts()
fig, ax = plt.subplots(figsize=(4.5, 4.5))
ax.pie(counts, labels=counts.index, autopct=lambda p: f"{p:.0f}%\n({p * counts.sum() / 100:.0f})",
       pctdistance=0.7, textprops=dict(fontsize=9))
ax.set_title("Ticket types")
print(counts.to_dict())
fig.savefig("tickets.png")
fig.savefig("pie_pandas_series_80dpi.png", dpi=80)
print(fig.get_size_inches(), len(fig.axes))
plt.close(fig)
