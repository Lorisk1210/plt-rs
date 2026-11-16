# Conversion rates shown as percentages on the y axis with PercentFormatter.
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

weeks = list(range(1, 11))
rate = [0.021, 0.025, 0.024, 0.031, 0.029, 0.035, 0.038, 0.036, 0.041, 0.044]
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(weeks, rate, marker="o")
ax.yaxis.set_major_formatter(PercentFormatter(xmax=1, decimals=1))
ax.set_ylim(0, 0.05)
ax.set_xlabel("week")
ax.set_title("Checkout conversion")
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()])
ax2 = ax.twinx()
ax2.set_ylim(0, 500)
ax2.yaxis.set_major_formatter(PercentFormatter(500))
fig.savefig("conversion.png")
