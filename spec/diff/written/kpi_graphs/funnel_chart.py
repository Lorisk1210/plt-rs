# Conversion funnel as centred horizontal bars with stage counts and conversion rates.
import matplotlib.pyplot as plt

stages = [("visits", 10000), ("signups", 3200), ("trials", 1400), ("paid", 420)]
fig, ax = plt.subplots(figsize=(6, 3.5))
for i, (name, n) in enumerate(stages):
    ax.barh(i, n, left=-n / 2, color=plt.cm.Blues(0.9 - i * 0.18), height=0.8)
    ax.text(0, i, f"{name}: {n:,}", ha="center", va="center", color="white" if i < 2 else "k")
    if i:
        ax.text(5600, i - 0.5, f"{n / stages[i - 1][1]:.0%}", va="center", fontsize=8, color="0.3")
ax.invert_yaxis()
ax.axis("off")
print([round(stages[i][1] / stages[i - 1][1], 3) for i in range(1, 4)])
fig.savefig("funnel.png")
