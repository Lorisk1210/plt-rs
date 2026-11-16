# Pie chart of how a day is spent with labels and percentages.
import matplotlib.pyplot as plt

activities = ["sleep", "school", "homework", "sport", "games", "other"]
hours = [9, 7, 2, 1.5, 2, 2.5]
plt.figure(figsize=(5, 5))
plt.pie(hours, labels=activities, autopct="%1.0f%%", startangle=90)
plt.title("My day")
plt.axis("equal")
plt.savefig("day.png")
print(sum(hours))
plt.close()
print(plt.get_fignums())
