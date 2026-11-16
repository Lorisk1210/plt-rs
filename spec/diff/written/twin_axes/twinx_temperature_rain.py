# Climate chart: monthly rainfall as bars on the left axis, temperature as a line on a twin right axis.
import matplotlib.pyplot as plt

months = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"]
rain = [78, 61, 58, 47, 55, 52, 66, 72, 61, 70, 85, 89]
temp = [3.1, 3.8, 7.2, 10.5, 14.6, 17.8, 19.9, 19.4, 16.2, 11.8, 7.0, 4.0]

fig, ax = plt.subplots(figsize=(7, 4))
ax.bar(range(12), rain, color="#6baed6", label="rain")
ax.set_ylabel("precipitation [mm]", color="#2171b5")
ax.tick_params(axis="y", colors="#2171b5")
ax.set_xticks(range(12), months)
ax2 = ax.twinx()
ax2.plot(range(12), temp, color="#cb181d", marker="o", label="temperature")
ax2.set_ylabel("temperature [°C]", color="#cb181d")
ax2.tick_params(axis="y", colors="#cb181d")
ax2.set_ylim(0, 25)
lines = ax.get_legend_handles_labels()
lines2 = ax2.get_legend_handles_labels()
ax2.legend(lines[0] + lines2[0], lines[1] + lines2[1], loc="upper left")
ax.set_title("Climate of a city")
print(ax.get_ylim(), ax2.get_ylim())
fig.savefig("climate.png")
