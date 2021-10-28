import pyron
from collections import namedtuple

print(pyron.to_string({"a": 5}))
print(pyron.to_string([1,2,3,4]))
print(pyron.to_string(namedtuple("Point", ["x", "y"])(1,2)))