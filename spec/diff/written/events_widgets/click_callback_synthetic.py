# Record clicks on a plot with mpl_connect, driven by synthetic MouseEvents (testing a click handler).
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, ax = plt.subplots(figsize=(5, 4))
ax.plot([0, 10], [0, 10], alpha=0)
clicks = []


def on_click(event):
    if event.inaxes is ax:
        clicks.append((round(event.xdata, 2), round(event.ydata, 2), event.button))
        ax.plot(event.xdata, event.ydata, "rx")


cid = fig.canvas.mpl_connect("button_press_event", on_click)
fig.canvas.draw()
for xd, yd in [(2, 3), (5, 5), (8, 1)]:
    x, y = ax.transData.transform((xd, yd))
    MouseEvent("button_press_event", fig.canvas, x, y, button=1)._process()
fig.canvas.mpl_disconnect(cid)
x, y = ax.transData.transform((9, 9))
MouseEvent("button_press_event", fig.canvas, x, y, button=1)._process()
print(clicks)
fig.savefig("clicks.png")
