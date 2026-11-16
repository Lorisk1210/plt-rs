# plt.boxplot from a dict of lists (keys as labels), median line restyled through setp.
import matplotlib.pyplot as plt

data = {"Mon": [7, 8, 6, 9, 7, 8], "Tue": [5, 6, 7, 6, 5, 9], "Wed": [8, 9, 9, 10, 7, 8],
        "Thu": [6, 6, 5, 7, 6, 4], "Fri": [9, 10, 11, 9, 12, 10]}
plt.figure(figsize=(6, 3.5))
result = plt.boxplot(list(data.values()), tick_labels=list(data.keys()))
plt.setp(result["medians"], color="red", linewidth=2)
plt.setp(result["whiskers"], linestyle="--")
plt.setp(result["caps"], color="gray")
plt.ylabel("hours of sleep")
plt.grid(axis="y", alpha=0.3)
plt.title("Sleep per weekday")
plt.savefig("sleep.png")
print([round(w.get_ydata()[1], 2) for w in result["whiskers"]])
