# Axis.set_units/get_units with the date converter's units (a tz) and the category converter's mapping.
import datetime as dt
from zoneinfo import ZoneInfo
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot([dt.datetime(2024, 1, 1, h, tzinfo=ZoneInfo("UTC")) for h in (0, 6, 12)], [1, 2, 3])
a.xaxis.set_units(ZoneInfo("Asia/Tokyo"))
print(a.xaxis.get_units())
b.bar(["x", "y"], [1, 2])
print(type(b.xaxis.get_units()).__name__, dict(b.xaxis.get_units()._mapping))
fig.autofmt_xdate()
fig.canvas.draw()
print([t.get_text() for t in a.get_xticklabels()][:2])
fig.savefig("units_set.png")
