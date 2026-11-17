# Which numbers new figures get after closing some, num= collisions with existing labels, and figure(num) with a different size.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

figs = [plt.figure() for _ in range(5)]
print([f.number for f in figs])
plt.close(figs[1])
plt.close(figs[3])
new = plt.figure()
print(new.number, plt.get_fignums())
new2 = plt.figure()
print(new2.number)
same = plt.figure(1, figsize=(10, 10))
print(same is figs[0], tuple(same.get_size_inches()))
lab = plt.figure("5")
print(lab.number, lab.get_label(), lab is figs[4])
num5 = plt.figure(5)
print(num5 is figs[4], num5.get_label())
plt.close("all")
fresh = plt.figure()
print(fresh.number)
big = plt.figure(100)
nxt = plt.figure()
print(big.number, nxt.number)
plt.close("all")
fig, ax = plt.subplots(num="dashboard")
try:
    plt.subplots(num="dashboard")
except ValueError as e:
    print("ValueError", str(e)[:60])
print(fig.get_label(), len(fig.axes))
fig3, ax3 = plt.subplots(num="dashboard", clear=True)
print(fig3 is fig, len(fig.axes))
print(plt.get_fignums(), plt.get_figlabels())
plt.close("all")
