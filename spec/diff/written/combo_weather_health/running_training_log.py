# Training log: weekly distance bars stacked by run type, pace line on twin axis (inverted, mm:ss), and a heart-rate zone pie.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

rng = np.random.default_rng(158)
weeks = np.arange(1, 13)
easy, tempo, long = rng.uniform(10, 20, 12), rng.uniform(5, 10, 12), np.linspace(12, 30, 12)
pace = 330 - weeks * 3 + rng.normal(0, 5, 12)
fig, (a, b) = plt.subplots(1, 2, figsize=(12, 4), gridspec_kw={"width_ratios": [2.5, 1]})
a.bar(weeks, easy, label="easy")
a.bar(weeks, tempo, bottom=easy, label="tempo")
a.bar(weeks, long, bottom=easy + tempo, label="long")
a.legend(loc="upper left")
tw = a.twinx()
tw.plot(weeks, pace, "ko-")
tw.invert_yaxis()
tw.yaxis.set_major_formatter(FuncFormatter(lambda s, _: f"{int(s // 60)}:{int(s % 60):02d}"))
b.pie([40, 30, 20, 10], labels=["Z1", "Z2", "Z3", "Z4"], autopct="%d%%", colors=plt.cm.Reds(np.linspace(0.3, 0.9, 4)))
fig.tight_layout()
fig.canvas.draw()
print(round(float((easy + tempo + long).sum()), 2), [t.get_text() for t in tw.get_yticklabels()][:3])
fig.savefig("training.png", dpi=50)
