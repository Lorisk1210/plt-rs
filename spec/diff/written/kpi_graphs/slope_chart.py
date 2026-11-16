# Slope chart: before/after values per item connected by lines, coloured by direction.
import matplotlib.pyplot as plt

items = {"A": (20, 35), "B": (40, 30), "C": (25, 28), "D": (50, 52), "E": (35, 15)}
fig, ax = plt.subplots(figsize=(4, 5))
for name, (a, b) in items.items():
    color = "green" if b > a else "red"
    ax.plot([0, 1], [a, b], "o-", color=color)
    ax.text(-0.05, a, f"{name} {a}", ha="right", va="center")
    ax.text(1.05, b, f"{b}", ha="left", va="center")
ax.set_xticks([0, 1], ["2023", "2024"])
ax.set_xlim(-0.5, 1.4)
ax.spines[["left", "right", "top"]].set_visible(False)
ax.set_yticks([])
print(sum(b > a for a, b in items.values()))
fig.savefig("slope.png")
