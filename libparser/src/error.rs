use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibError {

    #[error("File loading failed: {0}, error: {1}")]
    LoadError(String, String),

    #[error("Function not found: {0}")]
    FuncNotFound(String),

    #[error("Parameter not found: {0}")]
    ParamError(String),

    #[error("Size error: expect {0} but got {1}")]
    SizeError(usize, usize),
}