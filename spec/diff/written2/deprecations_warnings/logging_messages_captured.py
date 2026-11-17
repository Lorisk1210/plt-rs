# matplotlib's logging output captured with a handler: findfont misses, imshow clipping, and set_loglevel changing what is seen.
import logging
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


class Collect(logging.Handler):
    def __init__(self):
        super().__init__()
        self.records = []

    def emit(self, record):
        self.records.append((record.name, record.levelname, record.getMessage().splitlines()[0][:70]))


handler = Collect()
logger = logging.getLogger("matplotlib")
logger.addHandler(handler)
fig, ax = plt.subplots()
ax.imshow(np.full((2, 2, 3), 1.5))
ax.text(0, 0, "x", family="No Such Font Family")
fig.canvas.draw()
for rec in handler.records:
    print(rec)
handler.records.clear()
matplotlib.set_loglevel("error")
ax.imshow(np.full((2, 2, 3), -1.0))
fig.canvas.draw()
print(len(handler.records), logger.level, logging.getLevelName(logger.getEffectiveLevel()))
matplotlib.set_loglevel("info")
ax.imshow(np.full((2, 2, 3), 9.0))
fig.canvas.draw()
print(sorted({r[1] for r in handler.records}), len(handler.records) > 0)
print(sorted(name for name in logging.root.manager.loggerDict if name.startswith("matplotlib."))[:5])
logger.removeHandler(handler)
matplotlib.set_loglevel("warning")
print(logging.getLevelName(logger.level))
