# Typos in rcParams keys and style names: errors users get, and a font family that does not exist.
import warnings
import logging
import matplotlib as mpl
import matplotlib.pyplot as plt

warnings.simplefilter("always")
for key, value in [("lines.linewidht", 2), ("axes.titlesize", "hugee")]:
    try:
        mpl.rcParams[key] = value
    except (KeyError, ValueError) as err:
        print(type(err).__name__, str(err)[:80])
try:
    plt.style.use("seaborn")
except OSError as err:
    print("OSError:", str(err)[:80])
logging.basicConfig(level=logging.WARNING, format="%(levelname)s %(name)s: %(message)s")
fig, ax = plt.subplots(figsize=(4, 3))
ax.set_title("font that is missing", family="NoSuchFontFamily")
fig.savefig("typos.png")
