# Bar chart of pets owned in class from two lists, with colours and values on top.
import matplotlib.pyplot as plt

pets = ["dog", "cat", "fish", "hamster", "none"]
count = [9, 7, 3, 2, 5]
bars = plt.bar(pets, count, color=["brown", "orange", "blue", "gold", "gray"])
for bar, c in zip(bars, count):
    plt.text(bar.get_x() + bar.get_width() / 2, c + 0.1, str(c), ha="center")
plt.title("Pets in our class")
plt.ylabel("students")
plt.ylim(0, 10)
plt.savefig("pets.png")
print(sum(count))
