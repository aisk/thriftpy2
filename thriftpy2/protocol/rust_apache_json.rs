use pyo3::prelude::*;

#[pyclass]
pub struct TApacheJSONProtocol {
    trans: PyObject,
    req: Option<String>,
}

#[pymethods]
impl TApacheJSONProtocol {
    #[new]
    pub fn new(trans: PyObject) -> PyResult<Self> {
        Ok(TApacheJSONProtocol {
            trans: trans,
            req: None,
        })
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

    fn get_protocol(&self, trans: PyObject) -> PyResult<TApacheJSONProtocol> {
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
