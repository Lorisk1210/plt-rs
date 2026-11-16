# Strings that look like numbers become categories; converting to float changes the axis.
import matplotlib.pyplot as plt

x_str = ["10", "2", "33", "4"]
y = [1, 2, 3, 4]
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot(x_str, y, "o-")
a.set_title("strings: categories")
b.plot([float(v) for v in x_str], y, "o-")
b.set_title("floats: numeric")
print([t.get_text() for t in a.get_xticklabels()], [round(v, 2) for v in b.get_xlim()])
fig.savefig("string_numbers.png")
a.set_ylabel("rank")
b.set_ylabel("rank")
fig.tight_layout()
fig.savefig("string_numbers_tight.png")
