# Plot x and y lists with markers and a legend, then add a second line and change colours.
import matplotlib.pyplot as plt

hours = [1, 2, 3, 4, 5, 6]
coffee = [1, 2, 2, 3, 3, 4]
tea = [0, 1, 1, 1, 2, 2]
plt.plot(hours, coffee, marker="o", label="coffee")
plt.plot(hours, tea, marker="s", label="tea", color="green")
plt.legend()
plt.xlabel("hours awake")
plt.ylabel("cups")
plt.title("Drinks")
plt.savefig("drinks.png")
print([l.get_label() for l in plt.gca().get_lines()])
