# setp/getp introspection: list settable properties of a line and set several at once.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
lines = ax.plot([1, 2, 3], [1, 2, 3], [1, 2, 3], [3, 2, 1])
plt.setp(lines, linewidth=3, linestyle="--")
plt.setp(lines[0], color="r", marker="o")
print(plt.getp(lines[0], "linewidth"), plt.getp(lines[1], "linestyle"))
print(plt.getp(lines[0], "color"), lines[1].get_color())
plt.setp(ax.get_xticklabels(), fontsize=14, color="gray")
print(ax.get_xticklabels()[0].get_fontsize())
fig.savefig("setp.png")
ax.set_title("setp")
print(plt.getp(ax, "title"))
