# Two sliders (mean, sd) updating a histogram (re-drawn) and a PDF line; values set by code and the state printed.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider

rng = np.random.default_rng(100)
base = rng.normal(size=500)
fig, ax = plt.subplots(figsize=(6, 5))
fig.subplots_adjust(bottom=0.3)
x = np.linspace(-6, 6, 200)
pdf, = ax.plot(x, np.exp(-x ** 2 / 2) / np.sqrt(2 * np.pi), "r")
s_mu = Slider(fig.add_axes([0.15, 0.15, 0.7, 0.03]), "mean", -3, 3, valinit=0)
s_sd = Slider(fig.add_axes([0.15, 0.08, 0.7, 0.03]), "sd", 0.2, 3, valinit=1)
def update(_):
    mu, sd = s_mu.val, s_sd.val
    for p in list(ax.patches):
        p.remove()
    ax.hist(base * sd + mu, bins=30, density=True, color="C0", alpha=0.5)
    pdf.set_ydata(np.exp(-((x - mu) / sd) ** 2 / 2) / (sd * np.sqrt(2 * np.pi)))
    ax.relim(); ax.autoscale_view()
s_mu.on_changed(update); s_sd.on_changed(update)
update(None)
s_mu.set_val(1.5)
s_sd.set_val(0.6)
print(round(s_mu.val, 3), round(s_sd.val, 3), len(ax.patches), round(float(pdf.get_ydata().max()), 4))
fig.savefig("sliders.png", dpi=50)
