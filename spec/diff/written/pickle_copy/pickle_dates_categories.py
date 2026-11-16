# Pickle a figure with a date axis and a categorical axis; check formatters survive.
import pickle
import datetime as dt
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot([dt.date(2024, 1, d) for d in (1, 10, 20)], [1, 3, 2])
b.bar(["red", "green", "blue"], [3, 5, 4])
fig.autofmt_xdate()
try:
    fig2 = pickle.loads(pickle.dumps(fig))
except TypeError as err:
    print(type(err).__name__, err)
    b.remove()
    fig2 = pickle.loads(pickle.dumps(fig))
fig2.canvas.draw()
print(type(fig2.axes[0].xaxis.get_major_formatter()).__name__, len(fig2.axes))
fig2.savefig("dates_cats.png")
