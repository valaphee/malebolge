#![feature(try_blocks)]
#![feature(int_roundings)]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Windows error")]
    Windows(#[from] windows::core::Error),
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("PE error")]
    Pe(#[from] object::read::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod module;
pub mod process;
