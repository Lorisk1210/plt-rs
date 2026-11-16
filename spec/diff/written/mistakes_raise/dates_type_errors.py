# Mixing dates and floats on one axis, and unparseable date strings: the conversion errors.
import datetime as dt
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3))
ax.plot([dt.date(2024, 1, 1), dt.date(2024, 2, 1)], [1, 2])
try:
    ax.plot([1.5, 2.5], [1, 2])
    fig.canvas.draw()
    print("floats accepted", ax.get_xlim())
except (TypeError, ValueError) as err:
    print(type(err).__name__, str(err)[:80])
try:
    ax.set_xlim("2024-13-01", "2024-02-01")
except (ValueError, TypeError) as err:
    print(type(err).__name__, str(err)[:80])
fig.savefig("dates_err.png")
