use pyo3::{IntoPyObjectExt, prelude::*};
use pyo3::types::{PyList, PyBytes, PyString, PyNone};
use serde_json::{self, Value};

// Thrift type constants matching Python implementation
const T_TYPE_BOOL: i32 = 2;
const T_TYPE_BYTE: i32 = 3;
const T_TYPE_I8: i32 = 3; // Same as BYTE
const T_TYPE_I16: i32 = 6;
const T_TYPE_I32: i32 = 8;
const T_TYPE_I64: i32 = 10;
const T_TYPE_DOUBLE: i32 = 4;
const T_TYPE_STRING: i32 = 11;
const T_TYPE_BINARY: i32 = 11; // Same as string in Apache JSON
const T_TYPE_STRUCT: i32 = 12;
const T_TYPE_LIST: i32 = 15;
const T_TYPE_SET: i32 = 14;
const T_TYPE_MAP: i32 = 13;

#[pyclass]
pub struct TApacheJSONProtocol {
    trans: Py<PyAny>,
    req: Option<Value>,
}

#[pymethods]
impl TApacheJSONProtocol {
    #[new]
    pub fn new(trans: Py<PyAny>) -> PyResult<Self> {
        Ok(TApacheJSONProtocol {
            trans,
            req: None,
        })
    }

    fn read_message_begin(&mut self, py: Python) -> PyResult<Py<PyAny>> {
        if self.req.is_none() {
            self.load_data(py)?;
        }

        if let Some(ref req) = self.req {
            if let Some(array) = req.as_array() {
                if array.len() >= 4 {
                    let name_val = &array[1];
                    let ttype_val = &array[2];
                    let seqid_val = &array[3];

                    // Extract values with proper error handling
                    let name = name_val.as_str().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err("Invalid message name format")
                    })?;
                    let ttype = ttype_val.as_i64().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err("Invalid message type format")
                    })?;
                    let seqid = seqid_val.as_i64().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err("Invalid sequence ID format")
                    })?;

                    let py_name = name.into_pyobject(py)?.into_any();
                    let py_ttype = ttype.into_pyobject(py)?.into_any();
                    let py_seqid = seqid.into_pyobject(py)?.into_any();

                    let result = PyList::new(py, [py_name, py_ttype, py_seqid])?;
                    return Ok(result.into());
                } else {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Invalid message format: array too short"
                    ));
                }
            } else {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid message format: expected array"
                ));
            }
        }

        Err(pyo3::exceptions::PyValueError::new_err(
            "No data available to read message"
        ))
    }

    fn read_message_end(&self, _py: Python) -> PyResult<()> {
        Ok(())
    }

    fn skip(&self, _py: Python, _ttype: i32) -> PyResult<()> {
        Ok(())
    }

    fn read_struct<'py>(&mut self, py: Python<'py>, obj: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        if let Some(ref req) = self.req {
            if let Some(array) = req.as_array() {
                if array.len() >= 5 {
                    let data = &array[4];
                    return self.dict_to_thrift(py, data, obj);
                }
            }
        }

        Ok(obj.into())
    }
}

impl TApacheJSONProtocol {
    fn dict_to_thrift<'py>(&self, py: Python<'py>, data: &Value, base_type: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
        // Check if data is a basic Python type (string, number, boolean, null)
        if data.is_string() || data.is_number() || data.is_boolean() || data.is_null() {
            // Try to extract base_type as integer to compare with Thrift type constants
            if let Ok(base_type_int) = base_type.extract::<i32>() {
                // Handle integer types
                if base_type_int == T_TYPE_I8 || base_type_int == T_TYPE_I16 ||
                   base_type_int == T_TYPE_I32 || base_type_int == T_TYPE_I64 {
                    if data.is_string() {
                        let s = data.as_str().unwrap();
                        if let Ok(i) = s.parse::<i64>() {
                            return Ok(i.into_bound_py_any(py)?);
                        }
                    } else if data.is_i64() {
                        let i = data.as_i64().unwrap();
                        return Ok(i.into_bound_py_any(py)?);
                    } else if data.is_f64() {
                        let f = data.as_f64().unwrap();
                        return Ok((f as i64).into_bound_py_any(py)?);
                    }
                }

                // Handle binary type
                if base_type_int == T_TYPE_BINARY && T_TYPE_BINARY != T_TYPE_STRING {
                    if data.is_string() {
                        let s = data.as_str().unwrap();
                        // TODO: Implement base64 decoding
                        // For now, just return as bytes
                        return Ok(PyBytes::new(py, s.as_bytes()).into_bound_py_any(py)?);
                    }
                }

                // Handle boolean type
                if base_type_int == T_TYPE_BOOL {
                    if data.is_string() {
                        let s = data.as_str().unwrap().to_lowercase();
                        let bool_val = match s.as_str() {
                            "true" | "1" => true,
                            "false" | "0" => false,
                            _ => false,
                        };
                        return Ok(bool_val.into_bound_py_any(py)?);
                    } else if data.is_boolean() {
                        let b = data.as_bool().unwrap();
                        return Ok(b.into_bound_py_any(py)?);
                    } else if data.is_i64() {
                        let i = data.as_i64().unwrap();
                        return Ok((i != 0).into_bound_py_any(py)?);
                    }
                }
            }

            // Default handling: if base_type is not a specific type or we couldn't extract it,
            // just return the data converted to Python (equivalent to Python's "return data")
            if data.is_string() {
                let s = data.as_str().unwrap();
                return Ok(PyString::new(py, s).into_bound_py_any(py)?);
            } else if data.is_i64() {
                let i = data.as_i64().unwrap();
                return Ok(i.into_bound_py_any(py)?);
            } else if data.is_f64() {
                let f = data.as_f64().unwrap();
                return Ok(f.into_bound_py_any(py)?);
            } else if data.is_boolean() {
                let b = data.as_bool().unwrap();
                return Ok(b.into_bound_py_any(py)?);
            } else if data.is_null() {
                return Ok(PyNone::get(py).to_owned().into_any());
            }
        }

        // If not a basic type, return the base type (fallback)
        Ok(base_type.into())
    }

    fn load_data(&mut self, py: Python) -> PyResult<()> {
        let mut data = Vec::new();
        let mut l_braces = 0;
        let mut in_string = false;

        // Check if transport has getvalue method
        if let Ok(getvalue) = self.trans.bind(py).getattr("getvalue") {
            if getvalue.is_callable() {
                match getvalue.call0() {
                    Ok(value) => {
                        if let Ok(data_str) = value.extract::<String>() {
                            match serde_json::from_str(&data_str) {
                                Ok(parsed) => {
                                    self.req = Some(parsed);
                                    return Ok(());
                                }
                                Err(_) => {
                                    self.req = None;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        // Read byte by byte until we have balanced JSON
        loop {
            let read_method = self.trans.bind(py).getattr("read")?;
            let result = read_method.call1((1,))?;
            let byte_data: Vec<u8> = result.extract()?;

            if byte_data.is_empty() {
                break;
            }

            data.extend_from_slice(&byte_data);

            if byte_data[0] == b'"' && !data.ends_with(b"\\\"") {
                in_string = !in_string;
            }

            if !in_string {
                if byte_data[0] == b'[' {
                    l_braces += 1;
                } else if byte_data[0] == b']' {
                    l_braces -= 1;
                }
            }

            if l_braces == 0 {
                break;
            }
        }

        if !data.is_empty() {
            match String::from_utf8(data) {
                Ok(data_str) => {
                    match serde_json::from_str(&data_str) {
                        Ok(parsed) => {
                            self.req = Some(parsed);
                        }
                        Err(_) => {
                            self.req = None;
                        }
                    }
                }
                Err(_) => {
                    self.req = None;
                }
            }
        } else {
            self.req = None;
        }

        Ok(())
    }
}

#[pyclass]
pub struct TApacheJSONProtocolFactory;

#[pymethods]
impl TApacheJSONProtocolFactory {
    #[new]
    fn new() -> Self {
        TApacheJSONProtocolFactory
    }

    fn get_protocol(&self, trans: Py<PyAny>) -> PyResult<TApacheJSONProtocol> {
        TApacheJSONProtocol::new(trans)
    }
}

#[pymodule]
#[pyo3(name="rust_apache_json")]
fn rust_apache_json(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TApacheJSONProtocol>()?;
    m.add_class::<TApacheJSONProtocolFactory>()?;
    Ok(())
}