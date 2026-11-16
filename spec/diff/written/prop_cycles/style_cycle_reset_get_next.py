# Users ask how to get the next colour from the cycle: _get_lines.get_next_color, or draw a dummy line.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
line1, = ax.plot([0, 1], [0, 1])
next_color = ax._get_lines.get_next_color()
ax.fill_between([0, 1], [0, 0.5], color=next_color, alpha=0.3)
line3, = ax.plot([0, 1], [1, 0])
print(line1.get_color(), next_color, line3.get_color())
ax.set_prop_cycle(None)
line4, = ax.plot([0, 1], [0.5, 0.5])
print(line4.get_color())
fig.savefig("next_color.png")
