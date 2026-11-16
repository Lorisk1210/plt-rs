# Buttons, CheckButtons and RadioButtons wired to a plot and triggered by code.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import Button, CheckButtons, RadioButtons

x = np.linspace(0, 2 * np.pi, 100)
fig, ax = plt.subplots(figsize=(7, 4))
fig.subplots_adjust(left=0.3)
lines = {name: ax.plot(x, f(x), label=name)[0] for name, f in [("sin", np.sin), ("cos", np.cos)]}
check = CheckButtons(fig.add_axes([0.03, 0.6, 0.18, 0.2]), list(lines), [True, True])
check.on_clicked(lambda label: lines[label].set_visible(not lines[label].get_visible()))
radio = RadioButtons(fig.add_axes([0.03, 0.3, 0.18, 0.2]), ["red", "blue"])
radio.on_clicked(lambda c: [l.set_color(c) for l in lines.values()])
pressed = []
button = Button(fig.add_axes([0.03, 0.1, 0.18, 0.08]), "reset")
button.on_clicked(lambda event: pressed.append("reset"))
check.set_active(1)
radio.set_active(1)
print(check.get_status(), radio.value_selected, [l.get_color() for l in lines.values()])
fig.savefig("widgets.png")
