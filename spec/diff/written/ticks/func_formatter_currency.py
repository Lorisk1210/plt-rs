# Revenue axis formatted as $1.2M / $500K with a FuncFormatter, and thousands separators elsewhere.
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter, StrMethodFormatter


def money(value, pos):
    if abs(value) >= 1e6:
        return f"${value / 1e6:.1f}M"
    if abs(value) >= 1e3:
        return f"${value / 1e3:.0f}K"
    return f"${value:.0f}"


years = [2019, 2020, 2021, 2022, 2023, 2024]
revenue = [350_000, 420_000, 810_000, 1_250_000, 1_900_000, 2_600_000]
users = [12000, 15500, 30100, 52000, 80500, 112000]
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(9, 3.5))
ax1.bar(years, revenue)
ax1.yaxis.set_major_formatter(FuncFormatter(money))
ax2.plot(years, users, "o-")
ax2.yaxis.set_major_formatter(StrMethodFormatter("{x:,.0f}"))
fig.tight_layout()
fig.canvas.draw()
print([t.get_text() for t in ax1.get_yticklabels()])
print([t.get_text() for t in ax2.get_yticklabels()])
fig.savefig("revenue.png")
