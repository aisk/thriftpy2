use pyo3::prelude::*;
use pyo3::types::PyList;
use serde_json::{self, Value};
use base64::prelude::*;
use std::ffi::CString;

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

    fn read_message_begin(&mut self, py: Python) -> PyResult<Py<PyAny>> {
        if self.req.is_none() {
            self.load_data(py)?;
        }

        // TODO: Improve this.
        if let Some(ref req) = self.req {
            if let Some(array) = req.as_array() {
                if array.len() >= 4 {
                    let name_val = &array[1];
                    let ttype_val = &array[2];
                    let seqid_val = &array[3];

                    let py_name = name_val.as_str().unwrap_or("").into_pyobject(py)?.into_any();
                    let py_ttype = ttype_val.as_i64().unwrap_or(0).into_pyobject(py)?.into_any();
                    let py_seqid = seqid_val.as_i64().unwrap_or(0).into_pyobject(py)?.into_any();

                    let result = PyList::new(py, [py_name, py_ttype, py_seqid])?;
                    return Ok(result.into());
                }
            }
        }

        let empty_list = PyList::empty(py);
        Ok(empty_list.into())
    }

    fn read_message_end(&self, _py: Python) -> PyResult<()> {
        Ok(())
    }

    fn skip(&self, _py: Python, _ttype: i32) -> PyResult<()> {
        Ok(())
    }

    fn read_struct(&mut self, py: Python, obj: Py<PyAny>) -> PyResult<Py<PyAny>> {
        if self.req.is_none() {
            self.load_data(py)?;
        }

        if let Some(ref req) = self.req {
            if let Some(array) = req.as_array() {
                if array.len() >= 5 {
                    let data = &array[4];
                    return self.dict_to_thrift(py, data, obj);
                }
            }
        }

        Ok(obj)
    }
}

impl TApacheJSONProtocol {
    fn dict_to_thrift(&self, _py: Python, _data: &Value, base_obj: Py<PyAny>) -> PyResult<Py<PyAny>> {
        // Simple implementation that returns the base object
        // This is a placeholder that needs to be implemented properly
        Ok(base_obj)
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