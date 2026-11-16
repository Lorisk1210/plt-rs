# contour with a 1-D z, too-small grid, and mismatched X/Y: matplotlib's error texts.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
for args in [(np.arange(5),), (np.zeros((1, 5)),), (np.arange(3), np.arange(4), np.zeros((3, 3)))]:
    try:
        ax.contour(*args)
        print("ok")
    except (TypeError, ValueError) as err:
        print(type(err).__name__, str(err)[:90])
ax.contourf(np.add.outer(np.arange(5), np.arange(6)))
fig.savefig("contour_ok.png")
