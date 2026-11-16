# Saving with an unknown extension or format, and to a directory that does not exist.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([1, 2])
for name, kw in [("plot.xyz", {}), ("plot", {"format": "docx"}), ("missing_dir/plot.png", {})]:
    try:
        fig.savefig(name, **kw)
        print(name, "saved")
    except (ValueError, FileNotFoundError) as err:
        print(name, type(err).__name__, str(err)[:90])
fig.savefig("plot")
fig.savefig("plot.jpeg")
print("default name saved")
