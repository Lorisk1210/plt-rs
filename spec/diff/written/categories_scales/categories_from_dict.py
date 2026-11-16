# Users plot dict keys/values directly and sort categories; also numbers given as strings.
import matplotlib.pyplot as plt

counts = {"python": 42, "rust": 17, "go": 23, "java": 30}
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.2))
a.bar(counts.keys(), counts.values())
a.set_title("dict order")
items = sorted(counts.items(), key=lambda kv: kv[1], reverse=True)
b.barh([k for k, _ in items], [v for _, v in items], color="C1")
b.invert_yaxis()
b.set_title("sorted")
fig.tight_layout()
fig.savefig("languages.png")
fig2, ax2 = plt.subplots()
ax2.plot(["1", "10", "2", "20"], [1, 2, 3, 4], "o-")
print([t.get_text() for t in ax2.get_xticklabels()])
fig2.savefig("string_numbers.png")
