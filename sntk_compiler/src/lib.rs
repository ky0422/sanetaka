pub mod checker;
pub mod compiler;
pub mod helpers;
pub mod instruction;

use sntk_core::parser::{ast::Position, ParsingError};
use sntk_proc::ErrorFormat;
use std::fmt::{self, write};

#[derive(Debug, Clone)]
pub enum CompileError {
    ParsingError(Vec<ParsingError>),
    TypeError(TypeError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut messages = String::new();

        match self {
            CompileError::ParsingError(errors) => {
                for ParsingError { message, position } in errors {
                    write(
                        &mut messages,
                        format_args!("Parsing Error: {} at line {}, column {}\n", message, position.0, position.1),
                    )?;
                }
            }
            CompileError::TypeError(TypeError { message, position }) => {
                write(
                    &mut messages,
                    format_args!("Type Error: {} at line {}, column {}\n", message, position.0, position.1),
                )?;
            }
        }

        write!(f, "{}", messages.trim_end())
    }
}

#[derive(Debug, Clone, ErrorFormat)]
pub struct TypeError {
    pub message: String,
    pub position: Position,
}

macro_rules! messages {
    ($( $name:ident => $message:expr );*;) => {
        $(
            pub const $name: &str = $message;
        )*
    };
}

messages! {
    EXPECTED_DATA_TYPE => "Expected {0} type, got {1} instead";
    UNDEFINED_IDENTIFIER => "Undefined identifier: {0}";
    UNKNOWN_TYPE => "Unknown type: {0}";
    UNKNOWN_ARRAY_TYPE => "Unknown array type";
    UNEXPECTED_PARAMETER_LENGTH => "Unexpected parameter length";
}

#[macro_export]
macro_rules! type_error {
    ($msg:ident; $( $r:expr ),*; $position:expr) => {
        $crate::CompileError::TypeError(
            $crate::TypeError::new($msg, vec![$( format!("{}", $r) ),*], $position.clone())
        )
    };
}
