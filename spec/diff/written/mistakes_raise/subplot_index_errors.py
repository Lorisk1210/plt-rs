# Off-by-one subplot numbers and grids: plt.subplot(2, 2, 5), subplot(0), and indexing a 1-D axes array as 2-D.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 4))
for args in [(2, 2, 5), (2, 2, 0), (221,), (2, 2, 1.5)]:
    try:
        ax = fig.add_subplot(*args)
        print(args, "ok", ax.get_subplotspec().num1)
    except (ValueError, TypeError) as err:
        print(args, type(err).__name__, str(err)[:80])
fig2, axs = plt.subplots(1, 3)
try:
    axs[0, 1].plot([1, 2])
except IndexError as err:
    print("IndexError:", err)
axs[1].plot([1, 2])
fig.savefig("subplot_errors.png")
fig2.savefig("one_row.png")
