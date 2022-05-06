from typing import List, Sequence
import pyron
from collections import namedtuple
from dataclasses import dataclass

assert (
    pyron.to_string({"a": 5})
    == """{
    "a": 5,
}"""
)
assert (
    pyron.to_string([1, 2, 3, 4])
    == """[
    1,
    2,
    3,
    4,
]"""
)
assert (
    pyron.to_string(namedtuple("Point", ["x", "y"])(1, 2))
    == """Point(
    x: 1,
    y: 2,
)"""
)


@dataclass
class User:
    name: str
    age: int


@dataclass
class QueryResult:
    users: List[User]
    count: int


string = pyron.to_string(
    QueryResult(
        users=[
            User(name="John", age=30),
            User(name="Jane", age=25),
        ],
        count=2,
    )
)

assert (
    string
    == """QueryResult(
    users: [
        User(
            name: "John",
            age: 30,
        ),
        User(
            name: "Jane",
            age: 25,
        ),
    ],
    count: 2,
)"""
)
assert pyron.loads(string) == {
    "users": [{"name": "John", "age": 30}, {"name": "Jane", "age": 25}],
    "count": 2,
}

print(
    pyron.loads(
        'Schedule(key: "state.step/config.steps", schedule: [(0, 0.0005), "lin", (1, 0)])',
        preserve_structs=True,
    )
)

result = pyron.loads(string, preserve_class_names=True)
assert result == {
    "users": [{"name": "John", "age": 30, "!__name__": "User"}, {"name": "Jane", "age": 25, "!__name__": "User"}],
    "count": 2,
    "!__name__": "QueryResult",
}, result
