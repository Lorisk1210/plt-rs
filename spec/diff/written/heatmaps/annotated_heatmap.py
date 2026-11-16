# Annotated heat map of harvest per farmer and vegetable, with text colour depending on cell value.
import numpy as np
import matplotlib.pyplot as plt

veg = ["cucumber", "tomato", "lettuce", "asparagus", "potato"]
farmers = ["Farmer Joe", "Upland Bros.", "Smith Gardening", "Agrifun", "Organiculture"]
harvest = np.array([[0.8, 2.4, 2.5, 3.9, 0.0], [2.4, 0.0, 4.0, 1.0, 2.7], [1.1, 2.4, 0.8, 4.3, 1.9],
                    [0.6, 0.0, 0.3, 0.0, 3.1], [0.7, 1.7, 0.6, 2.6, 2.2]])
fig, ax = plt.subplots(figsize=(6, 5))
im = ax.imshow(harvest, cmap="YlGn")
ax.set_xticks(range(len(farmers)), labels=farmers, rotation=45, ha="right", rotation_mode="anchor")
ax.set_yticks(range(len(veg)), labels=veg)
for i in range(len(veg)):
    for j in range(len(farmers)):
        ax.text(j, i, harvest[i, j], ha="center", va="center", color="w" if harvest[i, j] > 2 else "k")
cbar = fig.colorbar(im, ax=ax)
cbar.ax.set_ylabel("t/year", rotation=-90, va="bottom")
ax.set_title("Harvest of local farmers (t/year)")
fig.tight_layout()
print(harvest.sum().round(2))
fig.savefig("harvest.png")
