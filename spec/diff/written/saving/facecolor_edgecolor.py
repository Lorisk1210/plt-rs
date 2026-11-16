# Dark-themed chart for a dashboard: the saved file must keep the dark figure background and a border.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 200)
fig = plt.figure(figsize=(5, 3), facecolor="#222831")
ax = fig.add_subplot()
ax.set_facecolor("#393e46")
ax.plot(x, np.sin(x) * np.exp(-x / 6), color="#00adb5", lw=2)
ax.tick_params(colors="#eeeeee")
for spine in ax.spines.values():
    spine.set_color("#eeeeee")
ax.set_title("Damped signal", color="#eeeeee")

fig.savefig("dark_default.png")
fig.savefig("dark_override.png", facecolor="white", edgecolor="red")
fig.savefig("dark_auto.png", facecolor="auto", edgecolor="auto")
print(fig.get_facecolor())
print(fig.get_edgecolor())
