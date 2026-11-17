# pyplot's figure registry: numbers, string labels, reuse by number and label, gcf switching, close by every kind of key.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

a = plt.figure()
b = plt.figure("named")
c = plt.figure(7)
d = plt.figure()
print(plt.get_fignums(), plt.get_figlabels(), [f.number for f in (a, b, c, d)], b.get_label())
print(plt.figure("named") is b, plt.figure(7) is c, plt.gcf() is c)
plt.figure(a)
print(plt.gcf() is a, plt.fignum_exists(7), plt.fignum_exists("named"), plt.fignum_exists(99))
plt.close("named")
print(plt.get_fignums(), plt.get_figlabels())
plt.close(7)
plt.close(d)
print(plt.get_fignums(), plt.gcf() is a)
e = plt.figure(num=3, figsize=(2, 2), clear=True)
e.add_subplot().plot([1, 2])
e2 = plt.figure(num=3, clear=True)
print(e2 is e, len(e2.axes))
plt.close("all")
print(plt.get_fignums())
f = plt.figure()
print(f.number)
try:
    plt.close(3.5)
except TypeError as err:
    print("TypeError", str(err)[:60])
plt.close("does not exist")
print(plt.get_fignums())
from matplotlib._pylab_helpers import Gcf
print(len(Gcf.figs), Gcf.get_active().num if Gcf.get_active() else None, Gcf.has_fignum(f.number))
