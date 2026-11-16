# Three stacked time series sharing the x axis, with only the bottom one labelled.
import numpy as np
import matplotlib.pyplot as plt

t = np.arange(0, 48, 0.5)
temperature = 15 + 6 * np.sin(2 * np.pi * (t - 6) / 24)
humidity = 60 - 15 * np.sin(2 * np.pi * (t - 6) / 24)
wind = 3 + 2 * np.abs(np.sin(2 * np.pi * t / 12))

fig, (ax1, ax2, ax3) = plt.subplots(3, 1, sharex=True, figsize=(7, 6))
ax1.plot(t, temperature, color="tab:red")
ax1.set_ylabel("°C")
ax2.plot(t, humidity, color="tab:blue")
ax2.set_ylabel("%")
ax3.plot(t, wind, color="tab:gray")
ax3.set_ylabel("m/s")
ax3.set_xlabel("hours")
ax3.set_xlim(0, 48)
fig.subplots_adjust(hspace=0.08)
print([ax.get_xlim() for ax in (ax1, ax2, ax3)])
print(ax1.get_shared_x_axes().joined(ax1, ax3))
fig.savefig("weather.png")
