# Pickle axes with date and category units and twinned axes: categories refuse to pickle, dates round-trip; draw the copies.
import datetime
import io
import pickle
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3), dpi=60)
days = [datetime.date(2024, 2, d) for d in (1, 10, 20)]
a1.plot(days, [1, 3, 2])
a1.xaxis.set_major_formatter(mdates.DateFormatter("%d %b"))
a2.bar(["apples", "pears", "plums"], [3, 1, 2])
tw = a2.twinx()
tw.plot(["apples", "plums"], [10, 20], "r-")
fig.canvas.draw()
try:
    pickle.dumps(fig)
    print("categories pickled")
except TypeError as e:
    print("TypeError", str(e)[:60])
a2.remove()
tw.remove()
try:
    blob = pickle.dumps(fig)
    print("after removal", len(blob) > 1000)
except TypeError as e:
    print("still TypeError", str(e)[:60])
fig2, (d1, d2) = plt.subplots(1, 2, figsize=(6, 3), dpi=60)
d1.plot(days, [1, 3, 2])
d1.xaxis.set_major_formatter(mdates.DateFormatter("%d %b"))
dt = d2.twinx()
d2.plot([datetime.datetime(2024, 1, 1, h) for h in range(5)], range(5))
dt.plot([1, 2], [3, 4])
fig2.canvas.draw()
before = [t.get_text() for t in d1.get_xticklabels()]
f2 = pickle.loads(pickle.dumps(fig2))
f2.canvas.draw()
b1, b2, btw = f2.axes
print(before == [t.get_text() for t in b1.get_xticklabels()], before[:2])
print(type(b1.xaxis.get_converter()).__name__, btw.get_shared_x_axes().joined(btw, b2))
b1.plot([datetime.date(2024, 3, 1)], [0])
print([round(v, 2) for v in b1.get_xlim()], [round(v, 2) for v in d1.get_xlim()])
print(type(b1.xaxis.get_major_formatter()).__name__, b1.xaxis.get_major_formatter().fmt)
buf = io.BytesIO()
f2.savefig(buf, format="png")
print(len(buf.getvalue()) > 1000)
