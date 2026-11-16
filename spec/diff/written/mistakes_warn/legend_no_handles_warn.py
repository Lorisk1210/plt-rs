# Calling legend() with no labelled artists warns "No artists with labels found"; empty legend results.
import warnings
import matplotlib.pyplot as plt

warnings.simplefilter("always")
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot([1, 2, 3])
a.legend()
b.plot([1, 2, 3], label="_nolegend_")
b.plot([3, 2, 1], label="visible")
b.legend()
print(a.get_legend(), [t.get_text() for t in b.get_legend().get_texts()])
fig.savefig("no_handles.png")
