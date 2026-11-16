# Users try to move a line from one axes to another; matplotlib refuses, so they recreate it.
import matplotlib.pyplot as plt

fig, (a, b) = plt.subplots(1, 2, figsize=(7, 3))
line, = a.plot([1, 2, 3], [3, 1, 2], "o-", color="teal")
try:
    b.add_line(line)
except (ValueError, RuntimeError) as err:
    print(type(err).__name__, err)
line.remove()
b.plot(*line.get_data(), color=line.get_color(), marker=line.get_marker())
print(len(a.lines), len(b.lines))
fig.savefig("moved.png")
