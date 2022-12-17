use crate::{
    checker::{custom_data_type, get_type_from_ir_expression, CustomTypes, DeclaredTypes},
    CompileError, TypeError, TypeErrorKind,
};
use sntk_core::parser::ast::{
    ArrayLiteral, AutoStatement, BlockExpression, BooleanLiteral, CallExpression, DataType, DeclareStatement, Expression, ExpressionStatement,
    FunctionLiteral, Identifier, IfExpression, IndexExpression, InfixExpression, LetStatement, NumberLiteral, Parameter, Position, PrefixExpression,
    Program, ReturnStatement, Statement, StringLiteral, TypeStatement, TypeofExpression,
};
use sntk_ir::instruction::{Instruction, InstructionType, IrExpression, LiteralValue};

#[derive(Debug)]
pub struct Compiler {
    pub program: Program,
    pub declares: DeclaredTypes,
    pub customs: CustomTypes,
}

pub type CompileResult<T> = Result<T, CompileError>;

pub trait CompilerTrait {
    fn new(program: Program) -> Self;
    fn new_with(program: Program, declares: DeclaredTypes, customs: CustomTypes) -> Self;
    fn compile_program(&mut self) -> CompileResult<Vec<Instruction>>;
    fn compile_statement(&mut self, statement: &Statement) -> CompileResult<Instruction>;
    fn compile_expression(&mut self, expression: &Expression, position: &Position) -> CompileResult<IrExpression>;
}

impl CompilerTrait for Compiler {
    #[inline]
    fn new(program: Program) -> Self {
        Self {
            program,
            declares: DeclaredTypes::new(None),
            customs: CustomTypes::new(None),
        }
    }

    #[inline]
    fn new_with(program: Program, declares: DeclaredTypes, customs: CustomTypes) -> Self {
        Self { program, declares, customs }
    }

    fn compile_program(&mut self) -> CompileResult<Vec<Instruction>> {
        let mut instructions = Vec::new();

        if !self.program.errors.is_empty() {
            return Err(CompileError::ParsingError(self.program.errors.clone()));
        }

        for statement in self.program.statements.clone().iter() {
            instructions.push(self.compile_statement(statement)?);
        }

        Ok(instructions)
    }

    fn compile_statement(&mut self, statement: &Statement) -> CompileResult<Instruction> {
        Ok(match statement {
            Statement::LetStatement(LetStatement {
                name,
                value,
                position,
                data_type,
            }) => {
                let value = self.compile_expression(value, position)?;
                let value_type = get_type_from_ir_expression(&value, &self.declares, &self.customs, Some(data_type), position)?;

                if data_type == &value_type {
                    return Err(TypeError::new(
                        TypeErrorKind::ExpectedDataType(data_type.to_string(), value_type.to_string()),
                        *position,
                    ));
                }

                self.declares.set(&name.value, data_type);

                Instruction::new(InstructionType::StoreName(name.value.clone(), value), *position)
            }
            Statement::AutoStatement(AutoStatement { name, value, position }) => {
                let value = self.compile_expression(value, position)?;

                self.declares.set(
                    &name.value,
                    &get_type_from_ir_expression(&value, &self.declares, &self.customs, None, position)?,
                );

                Instruction::new(InstructionType::StoreName(name.value.clone(), value), *position)
            }
            Statement::ReturnStatement(ReturnStatement { value, position }) => {
                Instruction::new(InstructionType::Return(self.compile_expression(value, position)?), *position)
            }
            Statement::TypeStatement(TypeStatement {
                name, data_type, position, ..
            }) => {
                self.customs.set(&name.value, data_type);

                Instruction::new(InstructionType::None, *position)
            }
            Statement::StructStatement(_) => unimplemented!(),
            Statement::DeclareStatement(DeclareStatement { name, data_type, position }) => {
                self.declares.set(&name.value, data_type);

                Instruction::new(InstructionType::None, *position)
            }
            Statement::ExpressionStatement(ExpressionStatement { expression, position }) => {
                Instruction::new(InstructionType::Expression(self.compile_expression(expression, position)?), *position)
            }
        })
    }

    fn compile_expression(&mut self, expression: &Expression, position: &Position) -> CompileResult<IrExpression> {
        let expression = match expression {
            Expression::Identifier(Identifier { value, .. }) => IrExpression::Identifier(value.clone()),
            Expression::BlockExpression(BlockExpression { statements, .. }) => {
                let mut instructions = Vec::new();

                for statement in statements.iter() {
                    instructions.push(self.compile_statement(statement)?);

                    if let Statement::ReturnStatement(_) = statement {
                        break;
                    }
                }

                IrExpression::Block(instructions)
            }
            Expression::PrefixExpression(PrefixExpression { operator, right, .. }) => {
                IrExpression::Prefix(operator.clone(), Box::new(self.compile_expression(right, position)?))
            }
            Expression::InfixExpression(InfixExpression { operator, left, right, .. }) => IrExpression::Infix(
                Box::new(self.compile_expression(left, position)?),
                operator.clone(),
                Box::new(self.compile_expression(right, position)?),
            ),
            Expression::IfExpression(IfExpression {
                condition,
                consequence,
                alternative,
                position,
            }) => IrExpression::If(
                Box::new(self.compile_expression(condition, position)?),
                Box::new(self.compile_expression(&Expression::BlockExpression(*consequence.clone()), position)?),
                Box::new(
                    alternative
                        .clone()
                        .map(|alternative| self.compile_expression(&Expression::BlockExpression(*alternative), position))
                        .transpose()?,
                ),
            ),
            Expression::FunctionLiteral(FunctionLiteral {
                parameters,
                body,
                return_type,
                position,
                ..
            }) => {
                let mut new_parameters = Vec::new();

                for (index, parameter @ Parameter { name, data_type, spread }) in parameters.iter().enumerate() {
                    let data_type = custom_data_type(data_type, &self.customs, position)?;

                    if *spread {
                        if index != parameters.len() - 1 {
                            return Err(TypeError::new(TypeErrorKind::SpreadParameterMustBeLast, *position));
                        }

                        self.declares.set(&name.value, &DataType::Array(Box::new(data_type.clone())));

                        new_parameters.push(Parameter {
                            name: name.clone(),
                            data_type: DataType::Array(Box::new(data_type.clone())),
                            spread: *spread,
                        });

                        break;
                    } else {
                        self.declares.set(&name.value, &data_type);
                        new_parameters.push(parameter.clone());
                    }
                }

                IrExpression::Literal(LiteralValue::Function(
                    new_parameters,
                    match self.compile_expression(&Expression::BlockExpression(body.clone()), position)? {
                        IrExpression::Block(instructions) => instructions,
                        _ => unreachable!(),
                    },
                    custom_data_type(return_type, &self.customs, position)?,
                    None,
                ))
            }
            Expression::CallExpression(CallExpression {
                function,
                arguments,
                position,
            }) => {
                let mut compiled_arguments = Vec::new();
                let function = self.compile_expression(function, position)?;
                let function_type = match get_type_from_ir_expression(&function, &self.declares, &self.customs, None, position)? {
                    DataType::Fn(function_type) => function_type,
                    _ => unreachable!(),
                };

                for (index, (argument, (_, spread))) in arguments.iter().zip(function_type.1.iter()).enumerate() {
                    if *spread {
                        compiled_arguments.push(self.compile_expression(
                            &Expression::ArrayLiteral(ArrayLiteral {
                                elements: arguments[index..].to_vec(),
                                position: *position,
                            }),
                            position,
                        )?);

                        break;
                    }

                    compiled_arguments.push(self.compile_expression(argument, position)?);
                }

                IrExpression::Call(Box::new(function), compiled_arguments)
            }
            Expression::TypeofExpression(TypeofExpression { expression, position }) => {
                let expression = self.compile_expression(expression, position)?;

                IrExpression::Literal(LiteralValue::String(
                    get_type_from_ir_expression(&expression, &self.declares, &self.customs, None, position)?.to_string(),
                ))
            }
            Expression::IndexExpression(IndexExpression { left, index, position }) => IrExpression::Index(
                Box::new(self.compile_expression(left, position)?),
                Box::new(self.compile_expression(index, position)?),
            ),
            Expression::StringLiteral(StringLiteral { value, .. }) => IrExpression::Literal(LiteralValue::String(value.clone())),
            Expression::NumberLiteral(NumberLiteral { value, .. }) => IrExpression::Literal(LiteralValue::Number(*value)),
            Expression::BooleanLiteral(BooleanLiteral { value, .. }) => IrExpression::Literal(LiteralValue::Boolean(*value)),
            Expression::ArrayLiteral(ArrayLiteral { elements, .. }) => {
                let mut elements_compiled = Vec::new();

                for element in elements.iter() {
                    elements_compiled.push(self.compile_expression(element, position)?);
                }

                IrExpression::Literal(LiteralValue::Array(elements_compiled))
            }
            Expression::StructLiteral(_) => todo!(),
        };

        get_type_from_ir_expression(&expression, &self.declares, &self.customs, None, position)?;

        Ok(expression)
    }
}
