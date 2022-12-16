use crate::{
    builtin::get_builtin_function,
    instruction::{Identifier, Instruction, InstructionType, IrExpression, LiteralValue},
    RuntimeError, RuntimeErrorKind,
};
use sntk_core::{parser::ast::Position, tokenizer::token::Tokens};
use std::{collections::HashMap, fmt};

#[derive(Clone, PartialEq)]
pub struct IrEnvironment {
    pub values: HashMap<Identifier, LiteralValue>,
    pub parent: Option<Box<IrEnvironment>>,
}

impl IrEnvironment {
    #[inline]
    pub fn new(parent: Option<&IrEnvironment>) -> Self {
        Self {
            values: HashMap::new(),
            parent: parent.map(|parent| Box::new(parent.clone())),
        }
    }

    pub fn get(&self, name: &Identifier) -> Option<LiteralValue> {
        match self.values.get(name) {
            Some(value) => Some(value.clone()),
            None => match &self.parent {
                Some(parent) => parent.get(name),
                None => None,
            },
        }
    }

    pub fn set(&mut self, name: &Identifier, value: &LiteralValue) {
        self.values.insert(name.to_string(), value.clone());
    }
}

impl fmt::Debug for IrEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ values: {:?}, parent: {:?} }}", self.values.keys(), self.parent)
    }
}

#[derive(Debug, Clone)]
pub struct IrInterpreter {
    pub instructions: Vec<Instruction>,
    pub environment: IrEnvironment,
}

pub type Result<T> = std::result::Result<T, crate::RuntimeError>;

pub trait IrInterpreterBase {
    fn new(instructions: Vec<Instruction>) -> Self;
    fn new_with_environment(instructions: Vec<Instruction>, environment: &IrEnvironment) -> Self;
    fn eval(&mut self) -> Result<()>;
    fn last(&mut self) -> Result<LiteralValue>;
}

pub trait IrInterpreterTrait {
    fn eval_instruction(&mut self, instruction: &Instruction) -> Result<()>;
    fn eval_expression(&mut self, expression: &IrExpression, position: &Position) -> Result<LiteralValue>;
}

impl IrInterpreterBase for IrInterpreter {
    fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            environment: IrEnvironment::new(None),
        }
    }

    fn new_with_environment(instructions: Vec<Instruction>, environment: &IrEnvironment) -> Self {
        Self {
            instructions,
            environment: environment.clone(),
        }
    }

    fn eval(&mut self) -> Result<()> {
        for instruction in self.instructions.clone().iter() {
            self.eval_instruction(instruction)?;
        }

        Ok(())
    }

    fn last(&mut self) -> Result<LiteralValue> {
        self.clone()
            .instructions
            .last()
            .map(|instruction| {
                let position = Position(instruction.position.0, instruction.position.1);
                match instruction.instruction.clone() {
                    InstructionType::Return(expression) => self.eval_expression(&expression, &position),
                    _ => Ok(LiteralValue::Boolean(false)),
                }
            })
            .unwrap_or(Ok(LiteralValue::Boolean(false)))
    }
}

impl IrInterpreterTrait for IrInterpreter {
    fn eval_instruction(&mut self, instruction: &Instruction) -> Result<()> {
        let position = Position(instruction.position.0, instruction.position.1);

        match instruction.instruction.clone() {
            InstructionType::StoreName(name, expression) => {
                let expression = self.eval_expression(&expression, &position)?;

                self.environment.set(&name, &expression);
            }
            InstructionType::Expression(expression) => {
                self.eval_expression(&expression, &position)?;
            }
            #[allow(unused_variables)]
            InstructionType::Return(expression) => {
                // println!("{:?}", self.eval_expression(&expression, position)?);
                // ^ for debugging. TODO: remove this.
            }
            InstructionType::None => {}
        }

        Ok(())
    }

    fn eval_expression(&mut self, expression: &IrExpression, position: &Position) -> Result<LiteralValue> {
        match expression {
            IrExpression::Identifier(name) => match self.environment.get(name) {
                Some(value) => Ok(value),
                None => Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable(name.to_string()), *position)),
            },
            IrExpression::Literal(value) => match value {
                LiteralValue::Array(array) => {
                    let array = array
                        .iter()
                        .map(|expression| self.eval_expression(expression, position))
                        .collect::<std::result::Result<Vec<LiteralValue>, RuntimeError>>()?
                        .iter()
                        .map(|value| IrExpression::Literal(value.clone()))
                        .collect();

                    Ok(LiteralValue::Array(array))
                }
                _ => Ok(value.clone()),
            },
            IrExpression::Block(block) => {
                let mut interpreter = IrInterpreter::new_with_environment(block.clone(), &IrEnvironment::new(Some(&self.environment)));
                interpreter.eval()?;
                Ok(interpreter.last()?)
            }
            IrExpression::If(condition, consequence, alternative) => {
                let condition = self.eval_expression(condition, position)?;

                match condition {
                    LiteralValue::Boolean(true) => self.eval_expression(consequence, position),
                    LiteralValue::Boolean(false) => match *alternative.clone() {
                        Some(alternative) => self.eval_expression(&alternative, position),
                        None => Ok(LiteralValue::Boolean(false)),
                    },
                    _ => unreachable!(),
                }
            }
            IrExpression::Call(function, arguments) => {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.eval_expression(argument, position))
                    .collect::<std::result::Result<Vec<_>, _>>()?;

                let function = match *function.clone() {
                    IrExpression::Identifier(name) => match self.environment.get(&name) {
                        Some(value) => value,
                        None => {
                            return match get_builtin_function(&name) {
                                Some(function) => Ok(function(arguments.iter().collect())),
                                None => Err(RuntimeError::new(RuntimeErrorKind::UndefinedVariable(name.to_string()), *position)),
                            };
                        }
                    },
                    _ => self.eval_expression(function, position)?,
                };

                let (parameters, body, mut environment) = match function {
                    LiteralValue::Function(parameters, block, _, environment) => (
                        parameters.iter().map(|parameter| parameter.name.value.clone()).collect::<Vec<_>>(),
                        block,
                        match environment {
                            Some(environment) => environment,
                            None => IrEnvironment::new(Some(&self.environment)),
                        },
                    ),
                    value => return Err(RuntimeError::new(RuntimeErrorKind::NotAFunction(value.to_string()), *position)),
                };

                for (parameter, argument) in parameters.iter().zip(arguments.iter()) {
                    environment.set(parameter, argument);
                }

                let mut interpreter = IrInterpreter::new_with_environment(body, &environment);
                interpreter.eval()?;

                Ok(match interpreter.last()? {
                    LiteralValue::Function(parameters, body, return_type, r_environment) => LiteralValue::Function(
                        parameters,
                        body,
                        return_type,
                        Some(IrEnvironment::new(Some(&match r_environment {
                            Some(environment) => environment,
                            None => environment,
                        }))),
                    ),
                    value => value,
                })
            }
            IrExpression::Index(left, index) => {
                let (left, index) = (self.eval_expression(left, position)?, self.eval_expression(index, position)?);

                match (left, index) {
                    (LiteralValue::Array(array), LiteralValue::Number(index)) => {
                        let index = index as usize;

                        match array.get(index) {
                            Some(value) => self.eval_expression(value, position),
                            None => Err(RuntimeError::new(RuntimeErrorKind::IndexOutOfBounds(index), *position)),
                        }
                    }
                    (left, _) => Err(RuntimeError::new(RuntimeErrorKind::NotAnArray(left.to_string()), *position)),
                }
            }
            IrExpression::Prefix(operator, right) => {
                let right = self.eval_expression(right, position)?;

                match (operator, right) {
                    (Tokens::Minus, LiteralValue::Number(right)) => Ok(LiteralValue::Number(-right)),
                    (Tokens::Bang, LiteralValue::Boolean(right)) => Ok(LiteralValue::Boolean(!right)),
                    (operator, _) => Err(RuntimeError::new(RuntimeErrorKind::InvalidOperator(operator.to_string()), *position)),
                }
            }
            IrExpression::Infix(left, operator, right) => {
                let (left, right) = (self.eval_expression(left, position)?, self.eval_expression(right, position)?);

                match (left, right) {
                    (LiteralValue::Number(left), LiteralValue::Number(right)) => match operator {
                        Tokens::Plus => Ok(LiteralValue::Number(left + right)),
                        Tokens::Minus => Ok(LiteralValue::Number(left - right)),
                        Tokens::Asterisk => Ok(LiteralValue::Number(left * right)),
                        Tokens::Slash => Ok(LiteralValue::Number(left / right)),
                        Tokens::EQ => Ok(LiteralValue::Boolean(left == right)),
                        Tokens::NEQ => Ok(LiteralValue::Boolean(left != right)),
                        Tokens::LT => Ok(LiteralValue::Boolean(left < right)),
                        Tokens::LTE => Ok(LiteralValue::Boolean(left <= right)),
                        Tokens::GT => Ok(LiteralValue::Boolean(left > right)),
                        Tokens::GTE => Ok(LiteralValue::Boolean(left >= right)),
                        _ => Err(RuntimeError::new(RuntimeErrorKind::InvalidOperator(operator.to_string()), *position)),
                    },
                    (LiteralValue::String(left), LiteralValue::String(right)) => match operator {
                        Tokens::Plus => Ok(LiteralValue::String(format!("{}{}", left, right))),
                        Tokens::EQ => Ok(LiteralValue::Boolean(left == right)),
                        Tokens::NEQ => Ok(LiteralValue::Boolean(left != right)),
                        Tokens::LT => Ok(LiteralValue::Boolean(left < right)),
                        Tokens::LTE => Ok(LiteralValue::Boolean(left <= right)),
                        Tokens::GT => Ok(LiteralValue::Boolean(left > right)),
                        Tokens::GTE => Ok(LiteralValue::Boolean(left >= right)),
                        _ => Err(RuntimeError::new(RuntimeErrorKind::InvalidOperator(operator.to_string()), *position)),
                    },
                    (LiteralValue::Boolean(left), LiteralValue::Boolean(right)) => match operator {
                        Tokens::EQ => Ok(LiteralValue::Boolean(left == right)),
                        Tokens::NEQ => Ok(LiteralValue::Boolean(left != right)),
                        _ => Err(RuntimeError::new(RuntimeErrorKind::InvalidOperator(operator.to_string()), *position)),
                    },
                    (left, right) => Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidOperands(left.to_string(), right.to_string(), operator.to_string()),
                        *position,
                    )),
                }
            }
        }
    }
}
