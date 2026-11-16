# A beginner's first plot: a list of numbers, a title, axis labels and save.
import matplotlib.pyplot as plt

temperatures = [12, 14, 13, 17, 19, 22, 21, 18, 15]
plt.plot(temperatures)
plt.title("Temperatures this week and a bit")
plt.xlabel("day")
plt.ylabel("degrees")
plt.savefig("first.png")
print(plt.gca().get_xlim(), plt.gca().get_ylim())
plt.grid()
plt.savefig("first_grid.png")
print(len(plt.gca().get_xgridlines()))
