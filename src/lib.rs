use pyo3::types::{PyDict, PyList, PyTuple};
use pyo3::{exceptions, prelude::*, wrap_pyfunction};

#[pyfunction()]
pub fn to_string(py: Python, value: &PyAny) -> PyResult<String> {
    let value = extract(py, value)?;
    value
        .to_string_pretty(
            ron::ser::PrettyConfig::default()
                .struct_names(true)
                .decimal_floats(true),
        )
        .map_err(|e| exceptions::PyValueError::new_err(format!("{}", e)))
}

#[pyfunction()]
pub fn load(py: Python, s: &str) -> PyResult<PyObject> {
    let value: ron::Value =
        ron::de::from_str(s).map_err(|e| exceptions::PyValueError::new_err(format!("{}", e)))?;
    try_val_to_py(&value, py)
}

#[pymodule]
fn pyron(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_string, m)?).unwrap();
    m.add_function(wrap_pyfunction!(load, m)?).unwrap();
    Ok(())
}

fn extract(py: Python, value: &PyAny) -> Result<ron::Value, PyErr> {
    if let Ok(dict) = value.cast_as::<PyDict>() {
        let mut map = ron::Map::new();
        for (key, value) in dict {
            map.insert(extract(py, key)?, extract(py, value)?);
        }
        Ok(ron::Value::Map(map))
    } else if let Ok(tuple) = value.cast_as::<PyTuple>() {
        if is_namedtuple(tuple) {
            extract_namedtuple(py, tuple)
        } else {
            let mut seq = vec![];
            for value in tuple.iter() {
                seq.push(extract(py, value)?);
            }
            Ok(ron::Value::Seq(seq))
        }
    } else if let Ok(list) = value.cast_as::<PyList>() {
        let mut seq = vec![];
        for value in list.iter() {
            seq.push(extract(py, value)?);
        }
        Ok(ron::Value::Seq(seq))
    } else if let Ok(str) = value.extract::<String>() {
        Ok(ron::Value::String(str))
    } else if let Ok(bool) = value.extract::<bool>() {
        Ok(ron::Value::Bool(bool))
    } else if let Ok(int) = value.extract::<i64>() {
        Ok(ron::Value::Number(ron::Number::Integer(int)))
    } else if let Ok(float) = value.extract::<f64>() {
        Ok(ron::Value::Number(ron::Number::from(float)))
    } else if PyModule::import(py, "dataclasses")?
        .call_method1("is_dataclass", (value,))?
        .extract::<bool>()?
    {
        extract_dataclass(py, value)
    } else {
        Err(exceptions::PyValueError::new_err(format!(
            "Unsupported type: {}",
            value.get_type().name()?
        )))
    }
}

fn is_namedtuple(value: &PyTuple) -> bool {
    let bases = match value.get_type().getattr("__bases__") {
        Ok(bases) => bases,
        Err(_) => return false,
    };
    let bases = match bases.cast_as::<PyTuple>() {
        Ok(bases) => bases,
        Err(_) => return false,
    };
    if bases.len() != 1 {
        return false;
    }
    // TODO: check that bases[0] is tuple
    let fields = match value.getattr("_fields") {
        Ok(fields) => fields,
        Err(_) => return false,
    };
    fields.cast_as::<PyTuple>().is_ok()
}

fn extract_namedtuple(py: Python, value: &PyTuple) -> Result<ron::Value, PyErr> {
    let name = value
        .getattr("__class__")?
        .getattr("__name__")?
        .extract::<String>()?;
    let mut s = ron::value::Struct::new(Some(name));
    for (name, value) in value
        .call_method("_asdict", (), None)?
        .cast_as::<PyDict>()?
    {
        let name = name.extract::<String>()?;
        let value = extract(py, value)?;
        s.insert(name, value);
    }
    Ok(ron::Value::Struct(s))
}

fn extract_dataclass(py: Python, value: &PyAny) -> Result<ron::Value, PyErr> {
    let name = value
        .getattr("__class__")?
        .getattr("__name__")?
        .extract::<String>()?;
    let mut s = ron::value::Struct::new(Some(name));
    // for field in mydataclass.__dataclass_fields__:
    //   value = getattr(mydataclass, field)
    //   ..
    for field in value
        .getattr("__dataclass_fields__")?
        .cast_as::<PyDict>()?
        .keys()
    {
        let field = field.extract::<String>()?;
        let value = value.getattr(&field)?;
        let value = extract(py, value)?;
        s.insert(field, value);
    }
    Ok(ron::Value::Struct(s))
}

fn try_val_to_py(value: &ron::Value, py: Python) -> PyResult<PyObject> {
    let p = match value {
        ron::Value::String(s) => s.into_py(py),
        ron::Value::Number(ron::Number::Float(f)) => f.get().into_py(py),
        ron::Value::Number(ron::Number::Integer(i)) => i.into_py(py),
        ron::Value::Bool(b) => b.into_py(py),
        ron::Value::Struct(s) => {
            let dict = PyDict::new(py);
            for (key, value) in s.iter() {
                dict.set_item(key, try_val_to_py(value, py)?)?;
            }
            dict.into()
        }
        ron::Value::Tuple(t) => {
            let mut elements = vec![];
            for value in t.iter() {
                elements.push(try_val_to_py(value, py)?);
            }

            PyTuple::new(py, elements).into()
        }
        ron::Value::Seq(s) => {
            let mut list = vec![];
            for value in s {
                list.push(try_val_to_py(value, py)?);
            }
            PyList::new(py, list).into()
        }
        ron::Value::Map(m) => {
            let dict = PyDict::new(py);
            for (key, value) in m.iter() {
                dict.set_item(try_val_to_py(key, py)?, try_val_to_py(value, py)?)?;
            }
            dict.into()
        }
        ron::Value::Char(c) => c.into_py(py),
        ron::Value::Option(Some(value)) => try_val_to_py(value.as_ref(), py)?,
        ron::Value::Option(None) | ron::Value::Unit => None::<()>.into_py(py),
    };
    Ok(p)
}
