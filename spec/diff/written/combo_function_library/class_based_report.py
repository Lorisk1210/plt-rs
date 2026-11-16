# A Report class that owns a figure, adds sections (text, chart, table) and saves; used for two reports.
import numpy as np
import matplotlib.pyplot as plt


class Report:
    def __init__(self, title, rows):
        self.fig, self.axes = plt.subplots(rows, 1, figsize=(7, 2.6 * rows))
        self.fig.suptitle(title, fontweight="bold")
        self.next = 0

    def _ax(self):
        ax = self.axes[self.next]
        self.next += 1
        return ax

    def text(self, body):
        ax = self._ax()
        ax.axis("off")
        ax.text(0, 0.5, body, va="center", wrap=True)

    def chart(self, x, y, kind="line"):
        ax = self._ax()
        getattr(ax, {"line": "plot", "bar": "bar", "area": "fill_between"}[kind])(x, y)
        return ax

    def table(self, rows, header):
        ax = self._ax()
        ax.axis("off")
        return ax.table(cellText=rows, colLabels=header, loc="center")

    def save(self, path):
        self.fig.tight_layout()
        self.fig.savefig(path, dpi=50)
        return [ax.get_title() for ax in self.axes]


rep = Report("Weekly", 3)
rep.text("Sales grew by 4% this week. Returns fell slightly.")
ax = rep.chart(np.arange(7), [5, 6, 6, 7, 8, 7, 9], kind="bar")
ax.set_title("orders")
rep.table([["mon", "5"], ["sun", "9"]], ["day", "orders"])
print(rep.save("weekly.png"))
rep2 = Report("Monthly", 2)
rep2.chart(np.arange(30), np.sqrt(np.arange(30)), kind="area").set_title("cumulative")
rep2.chart(np.arange(30), np.sin(np.arange(30) / 3)).set_title("cycle")
print(rep2.save("monthly.png"), rep2.next)
