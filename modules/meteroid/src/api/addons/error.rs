use std::error::Error;

use error_stack::Report;
use thiserror::Error;

use common_grpc_error_as_tonic_macros_impl::ErrorAsTonic;
use meteroid_store::errors::StoreError;

#[derive(Debug, Error, ErrorAsTonic)]
pub enum AddOnApiError {
    #[error("Missing argument: {0}")]
    #[code(InvalidArgument)]
    MissingArgument(String),

    #[error("Store error: {0}")]
    #[code(Internal)]
    StoreError(String, #[source] Box<dyn Error>),
}

impl From<Report<StoreError>> for AddOnApiError {
    fn from(value: Report<StoreError>) -> Self {
        let err = Box::new(value.into_error());
        Self::StoreError("Error in api add-on service".to_string(), err)
    }
}
