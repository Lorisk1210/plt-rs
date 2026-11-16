# Dates converted to floats by the axis, floats back to dates in an event handler, displayed in an annotation.
import datetime as dt
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from matplotlib.backend_bases import MouseEvent

days = [dt.date(2024, 6, d) for d in range(1, 31)]
fig, ax = plt.subplots(figsize=(8, 3.5))
ax.plot(days, range(30))
note = ax.annotate("", (0, 0), xytext=(10, 10), textcoords="offset points")
def on_click(e):
    if e.inaxes is ax:
        when = mdates.num2date(e.xdata)
        note.xy = (e.xdata, e.ydata)
        note.set_text(when.strftime("%d %b"))
fig.canvas.mpl_connect("button_press_event", on_click)
fig.canvas.draw()
x = mdates.date2num(dt.datetime(2024, 6, 15))
px, py = ax.transData.transform((x, 14))
MouseEvent("button_press_event", fig.canvas, px, py, 1)._process()
print(note.get_text(), round(note.xy[0], 4), type(ax.xaxis.get_major_formatter()).__name__)
fig.savefig("date_click.png", dpi=50)
