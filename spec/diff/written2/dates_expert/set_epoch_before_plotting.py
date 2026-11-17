# mdates.set_epoch before any date plotting (old 0000-12-31 epoch), date2num values under it, and the error once dates were used.
import datetime
import matplotlib
matplotlib.use("agg")
import matplotlib.dates as mdates

print(mdates.get_epoch())
mdates._reset_epoch_test_example()
mdates.set_epoch("0000-12-31T00:00:00")
print(mdates.get_epoch())
d = datetime.datetime(2024, 1, 1)
print(round(mdates.date2num(d), 4), mdates.num2date(mdates.date2num(d)).isoformat())
import matplotlib.pyplot as plt
fig, ax = plt.subplots(figsize=(5, 2), dpi=60)
ax.plot([d, d + datetime.timedelta(days=3)], [0, 1])
fig.canvas.draw()
print([round(v, 3) for v in ax.get_xlim()], [t.get_text() for t in ax.get_xticklabels()][:3])
try:
    mdates.set_epoch("1970-01-01")
except RuntimeError as e:
    print("RuntimeError", str(e)[:60])
print(mdates.get_epoch())
mdates._reset_epoch_test_example()
mdates.set_epoch("2000-01-01T12:00")
print(mdates.get_epoch(), round(mdates.date2num(d), 4))
print(mdates.num2date(0).isoformat(), mdates.num2date(1.5).isoformat())
mdates._reset_epoch_test_example()
mdates.set_epoch("1970-01-01T00:00:00")
print(round(mdates.date2num(d), 4))
fig.savefig("epoch.png")
print(mdates.datestr2num("2024-01-01"), mdates.date2num(mdates.num2date(19723.0)))
fig2, ax2 = plt.subplots()
ax2.plot([d], [1], "o")
fig2.canvas.draw()
print([round(v, 3) for v in ax2.get_xlim()])
print(mdates.DateConverter().default_units([d], ax2.xaxis), type(mdates.DateConverter().axisinfo(None, ax2.xaxis).majfmt).__name__)
