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

#[pyfunction(
    preserve_structs = "false",
    preserve_class_names = "false",
    print_errors = "true"
)]
pub fn load(
    py: Python,
    path: &str,
    preserve_structs: bool,
    preserve_class_names: bool,
    print_errors: bool,
) -> PyResult<PyObject> {
    let parse = ron_parser::load(path)?;
    if preserve_structs && preserve_class_names {
        return Err(exceptions::PyValueError::new_err(
            "preserve_structs and preserve_class_names cannot be true at the same time",
        ));
    }
    if !parse.errors.is_empty() {
        if print_errors {
            parse.emit();
        }
        return Err(exceptions::PyValueError::new_err(format!(
            "Fail to parse: {}",
            path
        )));
    }
    try_val_to_py(py, &parse.value, preserve_structs, preserve_class_names)
}

#[pyfunction(
    preserve_structs = "false",
    preserve_class_names = "false",
    print_errors = "true"
)]
pub fn loads(
    py: Python,
    s: &str,
    preserve_structs: bool,
    preserve_class_names: bool,
    print_errors: bool,
) -> PyResult<PyObject> {
    let value = match ron_parser::parse(s, None) {
        Ok(value) => value,
        Err(parse) => {
            if print_errors {
                parse.emit();
            }
            return Err(exceptions::PyValueError::new_err(format!(
                "Fail to parse: {}",
                s
            )));
        }
    };
    try_val_to_py(py, &value, preserve_structs, preserve_class_names)
}

#[pymodule]
fn pyron(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_string, m)?).unwrap();
    m.add_function(wrap_pyfunction!(load, m)?).unwrap();
    m.add_function(wrap_pyfunction!(loads, m)?).unwrap();
    Ok(())
}

fn extract(py: Python, value: &PyAny) -> Result<ron::Value, PyErr> {
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut map = ron::Map::new();
        for (key, value) in dict {
            map.insert(extract(py, key)?, extract(py, value)?);
        }
        Ok(ron::Value::Map(map))
    } else if let Ok(tuple) = value.downcast::<PyTuple>() {
        if is_namedtuple(tuple) {
            extract_namedtuple(py, tuple)
        } else {
            let mut seq = vec![];
            for value in tuple.iter() {
                seq.push(extract(py, value)?);
            }
            Ok(ron::Value::Tuple(seq))
        }
    } else if let Ok(list) = value.downcast::<PyList>() {
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
    } else if let Ok(None) = value.extract::<Option<PyObject>>() {
        Ok(ron::Value::Option(None))
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
    let bases = match bases.downcast::<PyTuple>() {
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
    fields.downcast::<PyTuple>().is_ok()
}

fn extract_namedtuple(py: Python, value: &PyTuple) -> Result<ron::Value, PyErr> {
    let name = value
        .getattr("__class__")?
        .getattr("__name__")?
        .extract::<String>()?;
    let mut s = ron::value::Struct::new(Some(name));
    for (name, value) in value
        .call_method("_asdict", (), None)?
        .downcast::<PyDict>()?
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
        .downcast::<PyDict>()?
        .keys()
    {
        let field = field.extract::<String>()?;
        let value = value.getattr(&*field)?;
        let value = extract(py, value)?;
        s.insert(field, value);
    }
    Ok(ron::Value::Struct(s))
}

fn try_val_to_py(
    py: Python,
    value: &ron_parser::Value,
    preserve_structs: bool,
    preserve_class_names: bool,
) -> PyResult<PyObject> {
    use ron_parser::Value;
    let p = match value {
        Value::String(s) => s.into_py(py),
        Value::Number(ron_parser::Number::Float(f)) => f.get().into_py(py),
        Value::Number(ron_parser::Number::Integer(i)) => i.into_py(py),
        Value::Bool(b) => b.into_py(py),
        Value::Struct(s) => {
            let dict = PyDict::new(py);
            for (key, value) in s.iter() {
                dict.set_item(
                    key,
                    try_val_to_py(py, value, preserve_structs, preserve_class_names)?,
                )?;
            }
            match &s.name {
                Some(name) if preserve_structs => {
                    let namedtuple = PyModule::import(py, "collections")?
                        .call_method1("namedtuple", (name.to_string(), dict.keys()))?;
                    namedtuple.call((), Some(dict))?.into()
                }
                Some(name) if preserve_class_names => {
                    dict.set_item("!__name__", name)?;
                    dict.into()
                }
                _ => dict.into(),
            }
        }
        Value::Tuple(name, t) => {
            let mut elements = vec![];
            for value in t.iter() {
                elements.push(try_val_to_py(
                    py,
                    value,
                    preserve_structs,
                    preserve_class_names,
                )?);
            }

            match name {
                Some(name) if preserve_structs => {
                    let namedtuple = PyModule::import(py, "collections")?.call_method1(
                        "namedtuple",
                        (
                            name.to_string(),
                            (0..t.len()).map(|i| format!("_{}", i)).collect::<Vec<_>>(),
                        ),
                    )?;
                    let dict = PyDict::new(py);
                    for (i, value) in t.iter().enumerate() {
                        dict.set_item(
                            format!("_{}", i),
                            try_val_to_py(py, value, preserve_structs, preserve_class_names)?,
                        )?;
                    }
                    namedtuple.call((), Some(dict))?.into()
                }
                Some(name) if preserve_class_names => {
                    let dict = PyDict::new(py);
                    for (i, value) in t.iter().enumerate() {
                        dict.set_item(
                            format!("_{}", i),
                            try_val_to_py(py, value, preserve_structs, preserve_class_names)?,
                        )?;
                    }
                    dict.set_item("!__name__", name)?;
                    dict.into()
                }
                _ => PyTuple::new(py, elements).into(),
            }
        }
        Value::Seq(s) => {
            let mut list = vec![];
            for value in s {
                list.push(try_val_to_py(
                    py,
                    value,
                    preserve_structs,
                    preserve_class_names,
                )?);
            }
            PyList::new(py, list).into()
        }
        Value::Map(m) => {
            let dict = PyDict::new(py);
            for (key, value) in m.iter() {
                dict.set_item(
                    try_val_to_py(py, key, preserve_structs, preserve_class_names)?,
                    try_val_to_py(py, value, preserve_structs, preserve_class_names)?,
                )?;
            }
            dict.into()
        }
        Value::Char(c) => c.into_py(py),
        Value::Option(Some(value)) => {
            try_val_to_py(py, value.as_ref(), preserve_structs, preserve_class_names)?
        }
        Value::Option(None) => None::<()>.into_py(py),
        Value::Unit => ().into_py(py),
        Value::Include(path) => {
            return Err(exceptions::PyValueError::new_err(format!(
                "Unresolved #include(\"{}\") directive",
                path
            )))
        }
    };
    Ok(p)
}
