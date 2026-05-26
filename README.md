# pyron

[![PyPI](https://img.shields.io/pypi/v/python-ron.svg?style=flat-square)](https://pypi.org/project/python-ron/)

Python bindings for the [Rusty Object Notation](https://github.com/ron-rs/ron).

## Development

Build and test locally with `uv`:

```bash
uv sync --group dev --no-install-project
cargo generate-lockfile
cargo check --locked --all-targets
rm -rf dist
uv run --no-sync maturin build --release --locked --out dist
PYTHON=$(uv run --no-sync python -c 'import sys; print(sys.executable)')
uv pip install --python "$PYTHON" --reinstall --no-deps dist/*.whl
uv run --no-sync python test.py
uv run --no-sync pytest
```

Here's a paste-ready README section:

## Usage

`python-ron` exposes a small API for converting between RON text and Python objects.

```python
import pyron
```

### Parse RON from a string

```python
import pyron

text = """
Config(
    name: "demo",
    enabled: true,
    steps: [
        (0, 0.0005),
        "lin",
        (1, 0),
    ],
)
"""

obj = pyron.loads(text)
print(obj)
```

By default, named RON structs are converted to plain Python dictionaries:

```python
>>> pyron.loads('Point(x: 1, y: 2)')
{'x': 1, 'y': 2}
```

### Preserve RON struct names

Use `preserve_class_names=True` when you need to retain the RON type name:

```python
>>> pyron.loads('Point(x: 1, y: 2)', preserve_class_names=True)
('Point', {'x': 1, 'y': 2})
```

Use `preserve_structs=True` to convert named RON structs to Python `namedtuple` instances:

```python
>>> point = pyron.loads('Point(x: 1, y: 2)', preserve_structs=True)
>>> point
Point(x=1, y=2)
>>> point.x
1
```

### Parse RON from a file

```python
import pyron

config = pyron.load("config.ron")
```

The same options accepted by `loads` are also accepted by `load`:

```python
config = pyron.load("config.ron", preserve_class_names=True)
```

### Serialize Python objects to RON

```python
import pyron

obj = {
    "name": "demo",
    "enabled": True,
    "steps": [
        (0, 0.0005),
        "lin",
        (1, 0),
    ],
}

text = pyron.to_string(obj)
print(text)
```

Supported Python input types include:

* `None`
* `bool`
* `int`
* `float`
* `str`
* `list`
* `tuple`
* `dict`
* dataclass instances
* `namedtuple` instances

### Round trip example

```python
import pyron

original = {
    "schedule": [
        (0, 0.0005),
        "lin",
        (1, 0),
    ],
}

text = pyron.to_string(original)
restored = pyron.loads(text)

assert restored == original
```
