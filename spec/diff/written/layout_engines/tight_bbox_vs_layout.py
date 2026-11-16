# Users mixing tight_layout() and bbox_inches='tight' with rotated tick labels on a bar chart.
import matplotlib.pyplot as plt

products = ["Ultra Widget Pro", "Basic Gadget", "Mega Sprocket XL", "Mini Doohickey", "Standard Thingamajig"]
units = [340, 520, 150, 610, 275]
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.bar(products, units, color="cadetblue")
ax.set_xticks(range(len(products)), products, rotation=45, ha="right")
ax.set_ylabel("units sold")
ax.set_title("Units per product")
fig.savefig("clipped.png")
fig.tight_layout()
fig.savefig("tight_layout.png")
fig.savefig("tight_both.png", bbox_inches="tight")
print([round(v, 3) for v in ax.get_position().bounds])
