# TextBox widget whose submit callback re-plots an expression; set_val triggers it by code.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import TextBox

x = np.linspace(-2, 2, 100)
fig, ax = plt.subplots(figsize=(6, 4))
fig.subplots_adjust(bottom=0.2)
line, = ax.plot(x, x ** 2)


def submit(expression):
    line.set_ydata(eval(expression, {"np": np, "x": x}))
    ax.relim()
    ax.autoscale_view()
    ax.set_title(expression)


box = TextBox(fig.add_axes([0.2, 0.05, 0.6, 0.075]), "y = ", initial="x ** 2")
box.on_submit(submit)
box.set_val("np.sin(3 * x)")
print(box.text, ax.get_title())
fig.savefig("textbox.png")
