# Figure numbers and labels as users read them: number, get_label, plt.figure(label) reuse, and canvas names.
import matplotlib.pyplot as plt

f1 = plt.figure("results")
f2 = plt.figure()
f3 = plt.figure(10, figsize=(3, 2))
print(f1.number, f1.get_label(), f2.number, repr(f2.get_label()), f3.number)
again = plt.figure("results")
print(again is f1, plt.get_figlabels(), plt.get_fignums())
f2.set_label("renamed")
print(plt.get_figlabels(), type(f1.canvas).__name__, f1.canvas.get_default_filename())
plt.figure("results")
plt.plot([1, 2, 3])
plt.savefig("results.png", dpi=40)
plt.close("results")
print(plt.get_fignums())
