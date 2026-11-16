# A Formatter subclass that writes durations as h:mm, used on a y axis of task times.
import matplotlib.pyplot as plt
from matplotlib.ticker import Formatter, MultipleLocator


class HourMinuteFormatter(Formatter):
    def __call__(self, x, pos=None):
        hours, minutes = divmod(int(round(x)), 60)
        return f"{hours}:{minutes:02d}"


tasks = ["build", "test", "deploy", "review"]
minutes = [95, 140, 35, 60]
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.bar(tasks, minutes)
ax.yaxis.set_major_locator(MultipleLocator(30))
ax.yaxis.set_major_formatter(HourMinuteFormatter())
fig.canvas.draw()
print([t.get_text() for t in ax.get_yticklabels()])
fig.savefig("durations.png")
