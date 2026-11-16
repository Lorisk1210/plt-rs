# Resize an existing figure after plotting (users often call set_size_inches before saving).
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
ax.plot([1, 2, 3, 4], [10, 20, 25, 30], "o-")
ax.set_title("Resized")
print("before", fig.get_size_inches(), fig.canvas.get_width_height())
fig.set_size_inches(8, 2)
print("after", fig.get_size_inches(), fig.canvas.get_width_height())
fig.set_size_inches((3, 3), forward=True)
fig.set_figwidth(5)
fig.set_figheight(2.5)
print(fig.get_figwidth(), fig.get_figheight())
fig.set_dpi(120)
print(fig.get_dpi(), fig.bbox.bounds)
fig.savefig("resized.png")
