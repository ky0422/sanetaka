use crate::{
    error::{CompileError, TypeError, EXPECTED_DATA_TYPE},
    helpers::{compile_block, literal_value, type_checked_array, type_checked_function},
    tc::{Type, TypeTrait},
    type_error,
};
use sntk_core::{
    parser::ast::{
        BlockExpression, BooleanLiteral, CallExpression, DataType, Expression, ExpressionStatement, FunctionLiteral, Identifier, IfExpression,
        IndexExpression, InfixExpression, LetStatement, NumberLiteral, PrefixExpression, Program, ReturnStatement, Statement, StringLiteral,
        TypeStatement,
    },
    tokenizer::token::Tokens,
};
use sntk_ir::{
    builtin::get_builtin,
    code::{BinaryOp, BinaryOpEq, Instruction, UnaryOp},
    interpreter::{Interpreter, InterpreterBase},
};

#[derive(Debug, Clone)]
pub struct Code(pub Vec<Instruction>);

impl Code {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push_instruction(&mut self, instruction: &Instruction) {
        self.0.push(instruction.clone());
    }
}

/// Compile the AST generated by `sntk_core` parser into Sanetaka IR.
#[derive(Debug)]
pub struct Compiler {
    pub program: Program,
    pub code: Code,
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Provides the basic methods of the compiler.
pub trait CompilerTrait {
    fn new(program: Program) -> Self;
    fn compile_program(&mut self) -> CompileResult<Interpreter>;
    fn compile_let_statement(&mut self, let_statement: &LetStatement) -> CompileResult<()>;
    fn compile_return_statement(&mut self, return_statement: &ReturnStatement) -> CompileResult<()>;
    fn compile_type_statement(&mut self, type_statement: &TypeStatement) -> CompileResult<()>;
    fn compile_expression(&mut self, expression: &Expression, data_type: Option<DataType>) -> CompileResult<()>;
}

impl CompilerTrait for Compiler {
    /// **Creates a new Compiler instance.**
    /// it takes an argument of type `Program`.
    fn new(program: Program) -> Self {
        Self { program, code: Code::new() }
    }

    /// Compile the AST generated by `sntk_core` parser into Sanetaka IR.
    fn compile_program(&mut self) -> CompileResult<Interpreter> {
        if !self.program.errors.is_empty() {
            return Err(CompileError::ParsingError(self.program.errors.clone()));
        }

        for statement in self.program.statements.clone() {
            match statement {
                Statement::LetStatement(statement) => self.compile_let_statement(&statement)?,
                Statement::ReturnStatement(statement) => self.compile_return_statement(&statement)?,
                Statement::TypeStatement(statement) => self.compile_type_statement(&statement)?,
                Statement::StructStatement(_) => unimplemented!(),
                Statement::ExpressionStatement(ExpressionStatement { expression, .. }) => self.compile_expression(&expression, None)?,
            };
        }

        Ok(Interpreter::new(self.code.clone().0))
    }

    /// Compile a `let` statement.
    ///
    /// `let x: number = 5;` to Sanetaka IR:
    /// ```
    /// Instruction:
    ///     0: LoadConst 5.0
    ///     1: StoreName "x"
    /// ```
    fn compile_let_statement(&mut self, let_statement: &LetStatement) -> CompileResult<()> {
        let LetStatement { name, value, data_type, .. } = let_statement;

        self.compile_expression(value, Some(data_type.clone()))?;
        self.code.push_instruction(&Instruction::StoreName(name.clone().value));

        Ok(())
    }

    /// Compile a `return` statement.
    ///
    /// `return 5;` to Sanetaka IR:
    /// ```
    /// Instruction:
    ///     0: LoadConst 5.0
    ///     1: Return
    /// ```
    fn compile_return_statement(&mut self, return_statement: &ReturnStatement) -> CompileResult<()> {
        let ReturnStatement { value, .. } = return_statement;

        self.compile_expression(value, None)?;
        self.code.push_instruction(&Instruction::Return);

        Ok(())
    }

    /// Compile a `type` statement.
    #[allow(unused_variables)]
    fn compile_type_statement(&mut self, type_statement: &TypeStatement) -> CompileResult<()> {
        todo!()
    }

    /// Compile an expression statement.
    fn compile_expression(&mut self, expression: &Expression, data_type: Option<DataType>) -> CompileResult<()> {
        macro_rules! match_type {
            ($type:expr; $e:expr; $pos:expr;) => {
                let data_type = match data_type {
                    Some(data_type) => data_type,
                    None => Type::get_data_type_from_expression($e, &$pos)?,
                };

                if !Type(data_type.clone()).eq_from_type(&Type($type)) {
                    return Err(type_error! { EXPECTED_DATA_TYPE; data_type, $type; $pos; });
                }
            };
        }

        match expression {
            Expression::BlockExpression(BlockExpression { statements, position }) => {
                let block = compile_block(statements.clone(), position)?;

                if let Some(data_type) = data_type {
                    if !Type(block.clone().1).eq_from_type(&Type(data_type.clone())) {
                        return Err(type_error! { EXPECTED_DATA_TYPE; data_type, block.1; position.clone(); });
                    }
                }

                self.code.push_instruction(&Instruction::Block(block.0));

                Ok(())
            }

            Expression::Identifier(Identifier { value, .. }) => {
                self.code.push_instruction(&Instruction::LoadName(value.clone()));

                Ok(())
            }

            Expression::NumberLiteral(NumberLiteral { position, .. }) => {
                match_type! { DataType::Number; expression; position.clone(); };

                self.code.push_instruction(&Instruction::LoadConst(literal_value(expression.clone())?));

                Ok(())
            }

            Expression::StringLiteral(StringLiteral { position, .. }) => {
                match_type! { DataType::String; expression; position.clone(); };

                self.code.push_instruction(&Instruction::LoadConst(literal_value(expression.clone())?));

                Ok(())
            }

            Expression::BooleanLiteral(BooleanLiteral { position, .. }) => {
                match_type! { DataType::Boolean; expression; position.clone(); };

                self.code.push_instruction(&Instruction::LoadConst(literal_value(expression.clone())?));

                Ok(())
            }

            Expression::ArrayLiteral(expression) => {
                self.code
                    .push_instruction(&Instruction::LoadConst(type_checked_array(expression, data_type.clone())?));

                Ok(())
            }

            Expression::FunctionLiteral(expression) => {
                self.code
                    .push_instruction(&Instruction::LoadConst(type_checked_function(expression, data_type.clone())?));

                Ok(())
            }

            Expression::StructLiteral(_) => {
                todo!()
            }

            Expression::PrefixExpression(PrefixExpression { operator, right, .. }) => {
                self.compile_expression(right, None)?;

                match operator {
                    Tokens::Minus => self.code.push_instruction(&Instruction::UnaryOp(UnaryOp::Minus)),
                    Tokens::Bang => self.code.push_instruction(&Instruction::UnaryOp(UnaryOp::Not)),
                    _ => panic!("Unknown operator: {}", operator),
                }

                Ok(())
            }

            Expression::InfixExpression(InfixExpression {
                left,
                operator,
                right,
                position,
            }) => {
                let left_data_type = Type::get_data_type_from_expression(left, position)?;
                let right_data_type = Type::get_data_type_from_expression(right, position)?;

                self.compile_expression(left, Some(right_data_type.clone()))?;
                self.compile_expression(right, Some(left_data_type.clone()))?;

                if let Some(data_type) = data_type {
                    if !Type(data_type.clone()).eq_from_type(&Type(left_data_type.clone())) {
                        return Err(type_error! { EXPECTED_DATA_TYPE; data_type, left_data_type; position.clone(); });
                    }
                }

                match operator {
                    Tokens::Plus => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Add)),
                    Tokens::Minus => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Sub)),
                    Tokens::Asterisk => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Mul)),
                    Tokens::Slash => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Div)),
                    Tokens::Percent => self.code.push_instruction(&Instruction::BinaryOp(BinaryOp::Mod)),
                    Tokens::EQ => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Eq)),
                    Tokens::NEQ => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Neq)),
                    Tokens::LT => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Lt)),
                    Tokens::GT => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Gt)),
                    Tokens::LTE => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Lte)),
                    Tokens::GTE => self.code.push_instruction(&Instruction::BinaryOpEq(BinaryOpEq::Gte)),
                    _ => panic!(),
                }

                Ok(())
            }

            Expression::CallExpression(CallExpression { function, arguments, .. }) => {
                for argument in arguments.clone() {
                    self.compile_expression(&argument, None)?;
                }

                match *function.clone() {
                    Expression::Identifier(Identifier { value, .. }) => match get_builtin(value.clone()) {
                        Some(_) => self.code.push_instruction(&Instruction::LoadGlobal(value)),
                        None => self.code.push_instruction(&Instruction::LoadName(value)),
                    },
                    Expression::FunctionLiteral(FunctionLiteral { .. }) | Expression::CallExpression(CallExpression { .. }) => {
                        self.compile_expression(function, None)?;
                    }
                    expression => panic!("Unknown function: {:?}", expression),
                }

                self.code.push_instruction(&Instruction::CallFunction(arguments.len()));

                Ok(())
            }

            Expression::IndexExpression(IndexExpression { .. }) => {
                todo!()
            }

            Expression::IfExpression(IfExpression {
                condition,
                consequence,
                alternative,
                position,
            }) => {
                self.compile_expression(condition, None)?;

                self.code.push_instruction(&Instruction::If(
                    compile_block(consequence.statements.clone(), position)?.0,
                    alternative
                        .clone()
                        .map(|expression| compile_block(expression.statements.clone(), position))
                        .transpose()?
                        .map(|block| block.0),
                ));

                Ok(())
            }
        }
    }
}
