# The units registry as a dict: register a converter, look it up for scalars, arrays, lists and masked arrays, then remove it.
import datetime
import decimal
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.units as munits
import matplotlib.dates  # registers date converters
import matplotlib.category  # registers string converters


class Tag(str):
    pass


class TagConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        return [len(v) for v in value] if np.iterable(value) and not isinstance(value, str) else len(value)


samples = {
    "float": 1.0, "int list": [1, 2], "date": datetime.date(2024, 1, 1), "datetime64": np.datetime64("2024-01-01"),
    "date list": [datetime.date(2024, 1, 1)], "str": "a", "str array": np.array(["a", "b"]), "bytes": b"x",
    "decimal": decimal.Decimal("1.5"), "masked": np.ma.masked_array([1, 2], [0, 1]), "empty": [], "tag": Tag("abc"),
    "obj array of dates": np.array([datetime.datetime(2024, 1, 1)], dtype=object),
}
for name, value in samples.items():
    conv = munits.registry.get_converter(value)
    print(name, None if conv is None else type(conv).__name__)
munits.registry[Tag] = TagConverter()
print(type(munits.registry.get_converter(Tag("abc"))).__name__, type(munits.registry.get_converter([Tag("x")])).__name__)
print(munits.registry[Tag].convert([Tag("ab"), Tag("abcd")], None, None))
del munits.registry[Tag]
print(Tag in munits.registry, type(munits.registry.get_converter(Tag("abc"))).__name__)
print(sorted(t.__name__ for t in munits.registry if t.__module__ in ("datetime", "builtins", "numpy")))
print(munits.ConversionInterface.is_numlike(3) if hasattr(munits.ConversionInterface, "is_numlike") else None)
print(munits._is_natively_supported(np.array([1.0])), munits._is_natively_supported(np.array(["a"])))
info = munits.AxisInfo(label="x", default_limits=(0, 1))
print(info.label, info.default_limits, info.majloc, info.minfmt)
