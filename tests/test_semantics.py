from __future__ import annotations

from collections import namedtuple
from dataclasses import dataclass

import pytest

import pyron


def test_loads_primitives_and_containers() -> None:
    data = pyron.loads(
        '''
        {
            "flag": true,
            "negative": -7,
            "integer": 42,
            "float": 1.5,
            "string": "hello",
            "list": [1, 2, 3],
            "tuple": (1, "x", false),
            "nested": {"value": 3},
        }
        '''
    )

    assert data == {
        "flag": True,
        "negative": -7,
        "integer": 42,
        "float": 1.5,
        "string": "hello",
        "list": [1, 2, 3],
        "tuple": (1, "x", False),
        "nested": {"value": 3},
    }


def test_tuple_keys_are_preserved_in_maps() -> None:
    data = pyron.loads(
        '''
        {
            "ruleset": {
                "cost_modifiers": {
                    (0, 1, 0, 0, 0, 0): 1.0,
                },
            },
        }
        '''
    )

    cost_modifiers = data["ruleset"]["cost_modifiers"]
    assert cost_modifiers == {(0, 1, 0, 0, 0, 0): 1.0}


def test_named_struct_default_loads_as_dict() -> None:
    data = pyron.loads(
        'Schedule(key: "state.step/config.steps", schedule: [(0, 0.0005), "lin", (1, 0)])'
    )

    assert data == {
        "key": "state.step/config.steps",
        "schedule": [(0, 0.0005), "lin", (1, 0)],
    }


def test_preserve_class_names_on_nested_structs() -> None:
    text = '''
    QueryResult(
        users: [
            User(name: "John", age: 30),
            User(name: "Jane", age: 25),
        ],
        count: 2,
    )
    '''

    assert pyron.loads(text, preserve_class_names=True) == {
        "users": [
            {"name": "John", "age": 30, "!__name__": "User"},
            {"name": "Jane", "age": 25, "!__name__": "User"},
        ],
        "count": 2,
        "!__name__": "QueryResult",
    }


def test_preserve_structs_returns_namedtuples() -> None:
    data = pyron.loads(
        'Schedule(key: "state.step/config.steps", schedule: [(0, 0.0005), "lin", (1, 0)])',
        preserve_structs=True,
    )

    assert data.__class__.__name__ == "Schedule"
    assert data.key == "state.step/config.steps"
    assert data.schedule == [(0, 0.0005), "lin", (1, 0)]
    assert data._asdict() == {
        "key": "state.step/config.steps",
        "schedule": [(0, 0.0005), "lin", (1, 0)],
    }


def test_tuple_struct_semantics() -> None:
    assert pyron.loads("Point(1, 2)") == (1, 2)
    assert pyron.loads("Point(1, 2)", preserve_class_names=True) == {
        "_0": 1,
        "_1": 2,
        "!__name__": "Point",
    }

    data = pyron.loads("Point(1, 2)", preserve_structs=True)
    assert data.__class__.__name__ == "Point"
    assert data.field0 == 1
    assert data.field1 == 2
    assert tuple(data) == (1, 2)


def test_python_namedtuple_to_ron_roundtrip() -> None:
    Point = namedtuple("Point", ["x", "y"])
    text = pyron.to_string(Point(1, 2))

    assert "Point" in text
    assert pyron.loads(text) == {"x": 1, "y": 2}

    preserved = pyron.loads(text, preserve_structs=True)
    assert preserved.__class__.__name__ == "Point"
    assert preserved.x == 1
    assert preserved.y == 2


@dataclass
class User:
    name: str
    age: int


@dataclass
class QueryResult:
    users: list[User]
    count: int


def test_python_dataclass_to_ron_roundtrip() -> None:
    value = QueryResult(
        users=[User(name="John", age=30), User(name="Jane", age=25)],
        count=2,
    )

    text = pyron.to_string(value)
    assert "QueryResult" in text
    assert "User" in text

    assert pyron.loads(text) == {
        "users": [{"name": "John", "age": 30}, {"name": "Jane", "age": 25}],
        "count": 2,
    }

    assert pyron.loads(text, preserve_class_names=True) == {
        "users": [
            {"name": "John", "age": 30, "!__name__": "User"},
            {"name": "Jane", "age": 25, "!__name__": "User"},
        ],
        "count": 2,
        "!__name__": "QueryResult",
    }


def test_plain_python_roundtrip() -> None:
    value = {
        "numbers": [1, 2, 3],
        "pair": ("left", "right"),
        "config": {"enabled": True, "threshold": 0.5},
    }

    assert pyron.loads(pyron.to_string(value)) == value


def test_load_reads_file(tmp_path) -> None:
    path = tmp_path / "config.ron"
    path.write_text('Config(enabled: true, values: [1, 2, 3])')

    assert pyron.load(str(path), preserve_class_names=True) == {
        "enabled": True,
        "values": [1, 2, 3],
        "!__name__": "Config",
    }


def test_error_cases() -> None:
    with pytest.raises(ValueError, match="Fail to parse RON"):
        pyron.loads("not valid ron )", print_errors=False)

    with pytest.raises(ValueError, match="cannot be true at the same time"):
        pyron.loads("Config(enabled: true)", preserve_structs=True, preserve_class_names=True)

    with pytest.raises(ValueError, match="Unsupported type"):
        pyron.to_string(object())
