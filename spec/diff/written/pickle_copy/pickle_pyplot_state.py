# Pickled pyplot figure registers with pyplot again on load (plt.get_fignums) and keeps its number.
import pickle
import matplotlib.pyplot as plt

fig = plt.figure(num=7, figsize=(4, 3))
plt.plot([1, 2, 3])
plt.title("figure seven")
blob = pickle.dumps(fig)
plt.close("all")
print(plt.get_fignums())
restored = pickle.loads(blob)
print(plt.get_fignums(), restored.number if hasattr(restored, "number") else None)
plt.figure(restored.number)
plt.plot([3, 2, 1])
print(len(plt.gca().lines))
plt.savefig("seven.png")
