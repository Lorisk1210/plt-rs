# Categorical x for bars and a date x for a line in the same figure, plus string categories on a twin y.
import datetime as dt
import numpy as np
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(11, 4))
teams = ["red", "blue", "green", "yellow"]
a.bar(teams, [12, 7, 9, 4], color=teams)
a.plot(teams, [10, 8, 8, 6], "ko-")
a.scatter(["blue", "purple"], [11, 3], marker="*", s=200, color="k")
days = [dt.date(2024, 5, d) for d in range(1, 11)]
b.plot(days, np.arange(10) ** 1.5)
tw = b.twinx()
tw.plot(days, ["low", "low", "mid", "mid", "high", "mid", "low", "high", "high", "mid"], "s", color="C1")
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in a.get_xticklabels()], [t.get_text() for t in tw.get_yticklabels()])
print(dict(a.xaxis.units._mapping), dict(tw.yaxis.units._mapping))
fig.savefig("cats_dates.png", dpi=50)
