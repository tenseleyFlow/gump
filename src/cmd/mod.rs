pub mod add;
pub mod clean;
pub mod edit;
pub mod import;
pub mod init;
pub mod list;
pub mod query;
pub mod remove;

use crate::db::DatabaseError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CmdError {
    #[error("{0}")]
    Database(#[from] DatabaseError),

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, CmdError>;
