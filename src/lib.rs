use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyModule, PyTuple};
use pyo3::{wrap_pyfunction, Bound, IntoPyObjectExt};
use ron2::{Map, NamedContent, Number, ToRon, Value};
use std::str::FromStr;

#[pyfunction]
pub fn to_string(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = extract(value)?;
    value.to_ron().map_err(py_value_error)
}

#[pyfunction(signature = (path, preserve_structs = false, preserve_class_names = false, print_errors = true))]
pub fn load(
    py: Python<'_>,
    path: &str,
    preserve_structs: bool,
    preserve_class_names: bool,
    print_errors: bool,
) -> PyResult<Py<PyAny>> {
    let source = std::fs::read_to_string(path)
        .map_err(|err| PyValueError::new_err(format!("Failed to read {path}: {err}")))?;
    loads_impl(
        py,
        &source,
        preserve_structs,
        preserve_class_names,
        print_errors,
    )
}

#[pyfunction(signature = (s, preserve_structs = false, preserve_class_names = false, print_errors = true))]
pub fn loads(
    py: Python<'_>,
    s: &str,
    preserve_structs: bool,
    preserve_class_names: bool,
    print_errors: bool,
) -> PyResult<Py<PyAny>> {
    loads_impl(py, s, preserve_structs, preserve_class_names, print_errors)
}

#[pymodule]
fn pyron(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_string, m)?)?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    Ok(())
}

fn loads_impl(
    py: Python<'_>,
    s: &str,
    preserve_structs: bool,
    preserve_class_names: bool,
    print_errors: bool,
) -> PyResult<Py<PyAny>> {
    if preserve_structs && preserve_class_names {
        return Err(PyValueError::new_err(
            "preserve_structs and preserve_class_names cannot be true at the same time",
        ));
    }

    let value = Value::from_str(s).map_err(|err| {
        if print_errors {
            eprintln!("{err}");
        }
        PyValueError::new_err(format!("Fail to parse RON: {err}"))
    })?;

    value_to_py(py, &value, preserve_structs, preserve_class_names)
}

fn extract(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut map = Map::with_capacity(dict.len());
        for (key, item) in dict.iter() {
            map.insert(extract(&key)?, extract(&item)?);
        }
        Ok(Value::Map(map))
    } else if let Ok(tuple) = value.cast::<PyTuple>() {
        if is_namedtuple(tuple) {
            extract_namedtuple(tuple)
        } else {
            let mut items = Vec::with_capacity(tuple.len());
            for item in tuple.iter() {
                items.push(extract(&item)?);
            }
            Ok(Value::Tuple(items))
        }
    } else if let Ok(list) = value.cast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(extract(&item)?);
        }
        Ok(Value::Seq(items))
    } else if let Ok(bytes) = value.cast::<PyBytes>() {
        Ok(Value::Bytes(bytes.as_bytes().to_vec()))
    } else if let Ok(string) = value.extract::<String>() {
        Ok(Value::String(string))
    } else if let Ok(boolean) = value.extract::<bool>() {
        Ok(Value::Bool(boolean))
    } else if let Ok(int) = value.extract::<i128>() {
        Ok(Value::Number(Number::from(int)))
    } else if let Ok(uint) = value.extract::<u128>() {
        Ok(Value::Number(Number::from(uint)))
    } else if let Ok(float) = value.extract::<f64>() {
        Ok(Value::Number(Number::from(float)))
    } else if value.is_none() {
        Ok(Value::Option(None))
    } else if is_dataclass(value)? {
        extract_dataclass(value)
    } else {
        let type_name = value.get_type().name()?.to_string();
        Err(PyValueError::new_err(format!(
            "Unsupported type: {type_name}"
        )))
    }
}

fn is_dataclass(value: &Bound<'_, PyAny>) -> PyResult<bool> {
    let dataclasses = PyModule::import(value.py(), "dataclasses")?;
    dataclasses
        .call_method1("is_dataclass", (value,))?
        .extract()
}

fn is_namedtuple(value: &Bound<'_, PyTuple>) -> bool {
    match value.getattr("_fields") {
        Ok(fields) => fields.cast::<PyTuple>().is_ok(),
        Err(_) => false,
    }
}

fn extract_namedtuple(value: &Bound<'_, PyTuple>) -> PyResult<Value> {
    let name = value
        .getattr("__class__")?
        .getattr("__name__")?
        .extract::<String>()?;
    let dict_any = value.call_method0("_asdict")?;
    let dict = dict_any.cast::<PyDict>()?;
    let mut fields = Vec::with_capacity(dict.len());
    for (key, item) in dict.iter() {
        fields.push((key.extract::<String>()?, extract(&item)?));
    }
    Ok(Value::Named {
        name,
        content: NamedContent::Struct(fields),
    })
}

fn extract_dataclass(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    let name = value
        .getattr("__class__")?
        .getattr("__name__")?
        .extract::<String>()?;
    let fields_any = value.getattr("__dataclass_fields__")?;
    let fields_dict = fields_any.cast::<PyDict>()?;
    let mut fields = Vec::with_capacity(fields_dict.len());
    for field in fields_dict.keys().iter() {
        let field = field.extract::<String>()?;
        let item = value.getattr(field.as_str())?;
        fields.push((field, extract(&item)?));
    }
    Ok(Value::Named {
        name,
        content: NamedContent::Struct(fields),
    })
}

fn value_to_py(
    py: Python<'_>,
    value: &Value,
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    match value {
        Value::Bool(v) => v.into_py_any(py),
        Value::Char(v) => v.to_string().into_py_any(py),
        Value::Number(v) => number_to_py(py, *v),
        Value::String(v) => v.into_py_any(py),
        Value::Bytes(v) => Ok(PyBytes::new(py, v).into_any().unbind()),
        Value::Unit => None::<()>.into_py_any(py),
        Value::Option(Some(v)) => value_to_py(py, v, preserve_structs, preserve_class_names),
        Value::Option(None) => None::<()>.into_py_any(py),
        Value::Seq(values) => sequence_to_py(py, values, preserve_structs, preserve_class_names),
        Value::Tuple(values) => tuple_to_py(py, values, preserve_structs, preserve_class_names),
        Value::Map(map) => map_to_py(py, map, preserve_structs, preserve_class_names),
        Value::Struct(fields) => {
            struct_to_py(py, None, fields, preserve_structs, preserve_class_names)
        }
        Value::Named { name, content } => {
            named_to_py(py, name, content, preserve_structs, preserve_class_names)
        }
    }
}

fn number_to_py(py: Python<'_>, value: Number) -> PyResult<Py<PyAny>> {
    match value {
        Number::I8(v) => v.into_py_any(py),
        Number::I16(v) => v.into_py_any(py),
        Number::I32(v) => v.into_py_any(py),
        Number::I64(v) => v.into_py_any(py),
        Number::I128(v) => v.into_py_any(py),
        Number::U8(v) => v.into_py_any(py),
        Number::U16(v) => v.into_py_any(py),
        Number::U32(v) => v.into_py_any(py),
        Number::U64(v) => v.into_py_any(py),
        Number::U128(v) => v.into_py_any(py),
        Number::F32(_) | Number::F64(_) => value.into_f64().into_py_any(py),
        _ => value.into_f64().into_py_any(py),
    }
}

fn sequence_to_py(
    py: Python<'_>,
    values: &[Value],
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    let mut items = Vec::with_capacity(values.len());
    for value in values {
        items.push(value_to_py(
            py,
            value,
            preserve_structs,
            preserve_class_names,
        )?);
    }
    Ok(PyList::new(py, items)?.into_any().unbind())
}

fn tuple_to_py(
    py: Python<'_>,
    values: &[Value],
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    let mut items = Vec::with_capacity(values.len());
    for value in values {
        items.push(value_to_py(
            py,
            value,
            preserve_structs,
            preserve_class_names,
        )?);
    }
    Ok(PyTuple::new(py, items)?.into_any().unbind())
}

fn map_to_py(
    py: Python<'_>,
    map: &Map,
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    for (key, value) in map.iter() {
        dict.set_item(
            value_to_py(py, key, preserve_structs, preserve_class_names)?,
            value_to_py(py, value, preserve_structs, preserve_class_names)?,
        )?;
    }
    Ok(dict.into_any().unbind())
}

fn struct_to_py(
    py: Python<'_>,
    name: Option<&str>,
    fields: &[(String, Value)],
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new(py);
    let mut keys = Vec::with_capacity(fields.len());
    let mut values = Vec::with_capacity(fields.len());

    for (key, value) in fields.iter() {
        let py_value = value_to_py(py, value, preserve_structs, preserve_class_names)?;
        dict.set_item(key, py_value.clone_ref(py))?;
        keys.push(key.clone());
        values.push(py_value);
    }

    match name {
        Some(name) if preserve_structs => make_namedtuple(py, name, &keys, values),
        Some(name) if preserve_class_names => {
            dict.set_item("!__name__", name)?;
            Ok(dict.into_any().unbind())
        }
        _ => Ok(dict.into_any().unbind()),
    }
}

fn named_to_py(
    py: Python<'_>,
    name: &str,
    content: &NamedContent,
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<Py<PyAny>> {
    match content {
        NamedContent::Unit => {
            if preserve_structs {
                make_namedtuple(py, name, &[], Vec::new())
            } else if preserve_class_names {
                let dict = PyDict::new(py);
                dict.set_item("!__name__", name)?;
                Ok(dict.into_any().unbind())
            } else {
                Ok(PyDict::new(py).into_any().unbind())
            }
        }
        NamedContent::Struct(fields) => struct_to_py(
            py,
            Some(name),
            fields,
            preserve_structs,
            preserve_class_names,
        ),
        NamedContent::Tuple(values) => {
            let mut items = Vec::with_capacity(values.len());
            for value in values {
                items.push(value_to_py(
                    py,
                    value,
                    preserve_structs,
                    preserve_class_names,
                )?);
            }
            if preserve_structs {
                let fields: Vec<String> = (0..values.len()).map(|i| format!("field{i}")).collect();
                make_namedtuple(py, name, &fields, items)
            } else if preserve_class_names {
                let dict = PyDict::new(py);
                for (i, item) in items.iter().enumerate() {
                    dict.set_item(format!("_{i}"), item)?;
                }
                dict.set_item("!__name__", name)?;
                Ok(dict.into_any().unbind())
            } else {
                Ok(PyTuple::new(py, items)?.into_any().unbind())
            }
        }
    }
}

fn make_namedtuple(
    py: Python<'_>,
    name: &str,
    fields: &[String],
    values: Vec<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    let collections = PyModule::import(py, "collections")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("rename", true)?;
    let safe_name = safe_python_identifier(name, "RonValue");
    let namedtuple = collections
        .getattr("namedtuple")?
        .call((safe_name, fields.to_vec()), Some(&kwargs))?;
    let args = PyTuple::new(py, values)?;
    Ok(namedtuple.call1(args)?.unbind())
}

fn safe_python_identifier(name: &str, fallback: &str) -> String {
    let mut chars = name.chars();
    let mut out = String::new();

    if let Some(first) = chars.next() {
        if first == '_' || first.is_ascii_alphabetic() {
            out.push(first);
        } else {
            out.push('_');
            if first.is_ascii_alphanumeric() {
                out.push(first);
            }
        }
    }

    for ch in chars {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() || matches!(out.as_str(), "False" | "None" | "True") {
        fallback.to_string()
    } else {
        out
    }
}

fn py_value_error(err: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(err.to_string())
}
