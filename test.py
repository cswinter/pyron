from typing import List, Sequence
import pyron
from collections import namedtuple
from dataclasses import dataclass

print(pyron.to_string({"a": 5}))
print(pyron.to_string([1, 2, 3, 4]))
print(pyron.to_string(namedtuple("Point", ["x", "y"])(1, 2)))


@dataclass
class User:
    name: str
    age: int


@dataclass
class QueryResult:
    users: List[User]
    count: int


print(
    pyron.to_string(
        QueryResult(
            users=[
                User(name="John", age=30),
                User(name="Jane", age=25),
            ],
            count=2,
        )
    )
)
