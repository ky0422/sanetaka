use crate::{compiler::CompileResult, TypeError, TypeErrorKind};
use sntk_core::{
    parser::ast::{DataType, FunctionType, Parameter, Position},
    tokenizer::token::Tokens,
};
use sntk_ir::instruction::{Identifier, InstructionType, IrExpression, LiteralValue};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredTypes {
    pub types: HashMap<Identifier, DataType>,
    pub parent: Option<Box<DeclaredTypes>>,
}

impl DeclaredTypes {
    #[inline]
    pub fn new(parent: Option<&DeclaredTypes>) -> Self {
        Self {
            types: HashMap::new(),
            parent: parent.map(|parent| Box::new(parent.clone())),
        }
    }

    pub fn get(&self, name: &Identifier) -> Option<DataType> {
        match self.types.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.parent {
                Some(parent) => parent.get(name),
                None => None,
            },
        }
    }

    #[inline]
    pub fn set(&mut self, name: &Identifier, value: &DataType) {
        self.types.insert(name.to_string(), value.clone());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomTypes {
    pub types: HashMap<Identifier, DataType>,
    pub parent: Option<Box<CustomTypes>>,
}

impl CustomTypes {
    #[inline]
    pub fn new(parent: Option<&CustomTypes>) -> Self {
        Self {
            types: HashMap::new(),
            parent: parent.map(|parent| Box::new(parent.clone())),
        }
    }

    pub fn get(&self, name: &Identifier) -> Option<DataType> {
        match self.types.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.parent {
                Some(parent) => parent.get(name),
                None => None,
            },
        }
    }

    #[inline]
    pub fn set(&mut self, name: &Identifier, value: &DataType) {
        self.types.insert(name.to_string(), value.clone());
    }
}

pub fn get_type_from_ir_expression(
    expression: &IrExpression,
    declares: &DeclaredTypes,
    customs: &CustomTypes,
    data_type: Option<&DataType>,
    position: &Position,
) -> CompileResult<DataType> {
    let data_type = match data_type {
        Some(data_type) => Some(custom_data_type(data_type, customs, position)?),
        None => None,
    };

    let result = match expression.clone() {
        IrExpression::Identifier(identifier) => match declares.get(&identifier) {
            Some(data_type) => Ok(data_type),
            None => Err(TypeError::new(TypeErrorKind::UndefinedIdentifier(identifier.to_string()), *position)),
        },
        IrExpression::Literal(literal) => get_type_from_literal_value(&literal, declares, customs, data_type.as_ref(), position),
        IrExpression::Block(block) => get_type_from_ir_expression(
            match block.last() {
                Some(instruction) => match instruction.instruction {
                    InstructionType::Return(ref expression) | InstructionType::StoreName(_, ref expression) => expression,
                    _ => return Ok(DataType::Boolean),
                },
                None => return Ok(DataType::Boolean),
            },
            declares,
            customs,
            data_type.as_ref(),
            position,
        ),
        IrExpression::If(condition, consequence, alternative) => {
            let condition_type = get_type_from_ir_expression(&condition, declares, customs, data_type.as_ref(), position)?;
            let consequence_type = get_type_from_ir_expression(&consequence, declares, customs, data_type.as_ref(), position)?;
            let alternative_type = match *alternative {
                Some(alternative) => get_type_from_ir_expression(&alternative, declares, customs, data_type.as_ref(), position)?,
                None => DataType::Boolean,
            };

            if condition_type == DataType::Boolean {
                if consequence_type == alternative_type {
                    Ok(consequence_type)
                } else {
                    Err(TypeError::new(
                        TypeErrorKind::ExpectedDataType(consequence_type.to_string(), alternative_type.to_string()),
                        *position,
                    ))
                }
            } else {
                Err(TypeError::new(
                    TypeErrorKind::ExpectedDataType(DataType::Boolean.to_string(), condition_type.to_string()),
                    *position,
                ))
            }
        }
        IrExpression::Call(function, arguments) => {
            let function_type = get_type_from_ir_expression(&function, declares, customs, data_type.as_ref(), position)?;

            match function_type {
                DataType::Fn(FunctionType(_, parameters, return_type)) => {
                    let mut arguments_len = arguments.len();

                    for (index, ((parameter, spread), argument)) in parameters.iter().zip(arguments.iter()).enumerate() {
                        let argument_type = get_type_from_ir_expression(argument, declares, customs, data_type.as_ref(), position)?;

                        if *spread {
                            if parameter != &argument_type {
                                return Err(TypeError::new(
                                    TypeErrorKind::ExpectedDataType(parameter.to_string(), argument_type.to_string()),
                                    *position,
                                ));
                            }

                            arguments_len = index + 1;

                            break;
                        }

                        if parameter != &argument_type {
                            return Err(TypeError::new(
                                TypeErrorKind::ExpectedDataType(parameter.to_string(), argument_type.to_string()),
                                *position,
                            ));
                        }
                    }

                    if parameters.len() != arguments_len {
                        return Err(TypeError::new(
                            TypeErrorKind::ExpectedArguments(parameters.len(), arguments.len()),
                            *position,
                        ));
                    }

                    Ok(*return_type)
                }
                _ => Err(TypeError::new(TypeErrorKind::NotCallable(function_type.to_string()), *position)),
            }
        }
        IrExpression::Index(left, index) => {
            let left_type = get_type_from_ir_expression(&left, declares, customs, data_type.as_ref(), position)?;
            let index_type = get_type_from_ir_expression(&index, declares, customs, data_type.as_ref(), position)?;

            match left_type {
                DataType::Array(data_type) => {
                    if index_type == DataType::Number {
                        Ok(*data_type)
                    } else {
                        Err(TypeError::new(
                            TypeErrorKind::ExpectedDataType(DataType::Number.to_string(), index_type.to_string()),
                            *position,
                        ))
                    }
                }
                _ => Err(TypeError::new(TypeErrorKind::NotIndexable(left_type.to_string()), *position)),
            }
        }
        IrExpression::Prefix(_, expression) => get_type_from_ir_expression(&expression, declares, customs, data_type.as_ref(), position),
        IrExpression::Infix(left, operator, right) => {
            let left_type = get_type_from_ir_expression(&left, declares, customs, data_type.as_ref(), position)?;
            let right_type = get_type_from_ir_expression(&right, declares, customs, data_type.as_ref(), position)?;

            match operator {
                Tokens::Plus | Tokens::Minus | Tokens::Asterisk | Tokens::Slash | Tokens::Percent => {
                    if left_type == DataType::Number && right_type == DataType::Number {
                        Ok(DataType::Number)
                    } else {
                        Err(TypeError::new(
                            TypeErrorKind::ExpectedDataType(DataType::Number.to_string(), left_type.to_string()),
                            *position,
                        ))
                    }
                }
                Tokens::EQ | Tokens::NEQ | Tokens::LT | Tokens::GT | Tokens::LTE | Tokens::GTE => {
                    if left_type == right_type {
                        Ok(DataType::Boolean)
                    } else {
                        Err(TypeError::new(
                            TypeErrorKind::ExpectedDataType(left_type.to_string(), right_type.to_string()),
                            *position,
                        ))
                    }
                }
                _ => unreachable!(),
            }
        }
    };

    custom_data_type(&result?, customs, position)
}

fn get_type_from_literal_value(
    literal: &LiteralValue,
    declares: &DeclaredTypes,
    customs: &CustomTypes,
    data_type: Option<&DataType>,
    position: &Position,
) -> CompileResult<DataType> {
    match literal {
        LiteralValue::Number(_) => Ok(DataType::Number),
        LiteralValue::String(_) => Ok(DataType::String),
        LiteralValue::Boolean(_) => Ok(DataType::Boolean),
        LiteralValue::Array(elements) => {
            let mut element_type = DataType::Unknown;

            for element in elements {
                let data_type = get_type_from_ir_expression(element, declares, customs, data_type, position)?;

                if element_type == DataType::Unknown {
                    element_type = data_type;
                } else if element_type != data_type {
                    return Err(TypeError::new(
                        TypeErrorKind::ExpectedDataType(element_type.to_string(), data_type.to_string()),
                        *position,
                    ));
                }
            }

            match data_type {
                Some(data_type) => {
                    if element_type == DataType::Unknown {
                        element_type = match data_type {
                            DataType::Array(data_type) => *data_type.clone(),
                            _ => unreachable!(),
                        };
                    }

                    if data_type == &DataType::Array(Box::new(element_type.clone())) {
                        return Err(TypeError::new(
                            TypeErrorKind::ExpectedDataType(data_type.to_string(), DataType::Array(Box::new(element_type)).to_string()),
                            *position,
                        ));
                    }
                }
                None => {
                    if element_type == DataType::Unknown {
                        return Err(TypeError::new(TypeErrorKind::UnknownArrayType, *position));
                    }
                }
            }

            Ok(DataType::Array(Box::new(element_type)))
        }
        LiteralValue::Function(parameters, body, return_type, _) => {
            let block_return_type = Box::new(get_type_from_ir_expression(
                &IrExpression::Block(body.clone()),
                &declares,
                customs,
                data_type,
                position,
            )?);

            let function_type = Ok(DataType::Fn(FunctionType(
                None,
                parameters
                    .iter()
                    .map(|Parameter { data_type, spread, .. }| (data_type.clone(), *spread))
                    .collect(),
                block_return_type.clone(),
            )))?;

            if return_type.clone() != *block_return_type {
                return Err(TypeError::new(
                    TypeErrorKind::ExpectedDataType(return_type.to_string(), block_return_type.to_string()),
                    *position,
                ));
            }

            if let Some(data_type) = data_type {
                if data_type != &function_type {
                    return Err(TypeError::new(
                        TypeErrorKind::ExpectedDataType(data_type.to_string(), function_type.to_string()),
                        *position,
                    ));
                }
            }

            Ok(function_type)
        }
    }
}

pub fn custom_data_type(data_type: &DataType, customs: &CustomTypes, position: &Position) -> CompileResult<DataType> {
    Ok(match data_type {
        DataType::Custom(name) => match customs.get(name) {
            Some(custom) => custom.clone(),
            None => return Err(TypeError::new(TypeErrorKind::UndefinedType(name.clone()), *position)),
        },
        DataType::Fn(FunctionType(generics, parameters, return_type)) => {
            let return_type = custom_data_type(return_type, customs, position)?;

            DataType::Fn(FunctionType(generics.clone(), parameters.clone(), Box::new(return_type)))
        }
        _ => data_type.clone(),
    })
}
