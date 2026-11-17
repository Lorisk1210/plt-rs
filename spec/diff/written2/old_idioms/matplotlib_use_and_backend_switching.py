# matplotlib.use / get_backend / switch_backend between non-interactive backends; rcParams['backend'] and the canvas class each time.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

print(matplotlib.get_backend())
fig = plt.figure()
print(type(fig.canvas).__name__)
for name in ("pdf", "svg", "ps", "cairo", "agg", "template"):
    try:
        plt.switch_backend(name)
        f = plt.figure()
        print(name, matplotlib.get_backend(), type(f.canvas).__name__, plt.rcParams["backend"])
    except Exception as e:
        print(name, type(e).__name__, str(e)[:60])
matplotlib.use("agg", force=True)
print(matplotlib.get_backend(), plt.get_backend())
try:
    matplotlib.use("not-a-backend")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(matplotlib.is_interactive(), plt.isinteractive())
plt.ion()
print(plt.isinteractive())
plt.ioff()
print(plt.isinteractive())
with plt.ioff():
    print(plt.isinteractive())
print(plt.isinteractive())
f = plt.figure()
f.add_subplot().plot([1])
f.savefig("after_switch.png")
print(type(f.canvas).__name__)
plt.close("all")
