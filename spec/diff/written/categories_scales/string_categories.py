# Plot directly against string categories: bars, then a line on the same categorical axis.
import matplotlib.pyplot as plt

fruits = ["apple", "banana", "cherry", "date"]
sold = [30, 45, 12, 22]
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.bar(fruits, sold, color="khaki")
ax.plot(fruits, [25, 40, 15, 20], "ro-", label="target")
ax.scatter(["banana", "elderberry"], [50, 10], marker="*", s=120, color="k")
ax.legend()
print(ax.get_xticks(), [t.get_text() for t in ax.get_xticklabels()])
print(ax.xaxis.get_units())
fig.savefig("fruits.png")
ax.set_ylabel("crates")
ax.set_title("Fruit sold")
