"""
The types of the numbers a program gets back and hands on (task NUM1).

matplotlib hands a program its own ints and numpy scalars back, computes
in the dtype of the input, and calls the program's code with numpy
scalars and arrays of the dtype it holds. A *row* names one number a
program reads (a getter, a plotting method's result, a function's) or one
argument of one of its callbacks, and its value describes the type
(`describe.describe`): ``int=1``, ``np.float64=0.5``,
``ndarray[int64,(3,)]=[1, 2, 3]``.

Sections: getters of values the program gave (G), every getter of drawn
artists (S) and every float property set and read back (P), values
matplotlib computes (R) and keys of what plotting methods return (K), and
the arguments of the program's callbacks (C).

``python -m number_types --record`` records matplotlib's rows in
``reference.json``; ``python/tests/test_num1.py`` holds pltrs to them,
less the rows of ``expected.json``.
"""
