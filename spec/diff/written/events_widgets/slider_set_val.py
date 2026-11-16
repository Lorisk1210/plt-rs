# Slider controlling a sine frequency, moved by code with set_val and on_changed callback.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider

t = np.linspace(0, 1, 500)
fig, ax = plt.subplots(figsize=(6, 4))
fig.subplots_adjust(bottom=0.25)
line, = ax.plot(t, np.sin(2 * np.pi * 3 * t))
ax_freq = fig.add_axes([0.15, 0.1, 0.7, 0.04])
freq = Slider(ax_freq, "freq", 0.5, 10, valinit=3, valstep=0.5)
changes = []


def update(val):
    line.set_ydata(np.sin(2 * np.pi * val * t))
    changes.append(val)


freq.on_changed(update)
freq.set_val(5.2)
freq.set_val(7)
print(changes, freq.val, freq.valtext.get_text())
fig.savefig("slider.png")
freq.reset()
print(freq.val)
