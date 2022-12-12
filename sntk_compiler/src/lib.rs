pub mod checker;
pub mod compiler;
pub mod helpers;

use sntk_core::parser::{ast::Position, ParsingError};
use std::{
    borrow::Cow,
    fmt::{self, write},
};

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

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub position: Position,
}

impl TypeError {
    pub fn new(message: &str, args: Vec<&str>, position: &Position) -> Self {
        let mut message = message.to_string();

        args.iter().enumerate().for_each(|(i, arg)| {
            message = message.replace(&format!("{{{}}}", i), arg);
        });

        Self {
            message,
            position: position.to_owned(),
        }
    }
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
    EXPECTED_ARGUMENTS => "Expected {0} arguments, got {1} instead";
    UNDEFINED_IDENTIFIER => "Undefined identifier: {0}";
    UNKNOWN_TYPE => "Unknown type: {0}";
    UNKNOWN_ARRAY_TYPE => "Unknown array type";
    UNEXPECTED_PARAMETER_LENGTH => "Unexpected parameter length";
    NOT_A_FUNCTION => "{0} is not a function";
}

pub fn type_error<T>(message: T, replacements: Cow<[&str]>, position: &Position) -> CompileError
where
    T: Into<String>,
{
    CompileError::TypeError(TypeError::new(&message.into(), replacements.into_owned(), position))
}
