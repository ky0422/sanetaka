use super::{ast::*, error::*};
use crate::{
    tokenizer::{lexer::*, token::*},
    *,
};

pub type ParseResult<T> = Result<T, ParsingError>;

pub trait ParserBase {
    fn new(lexer: Lexer) -> Self;
    fn next_token(&mut self);
    fn expect_token(&mut self, token_type: Tokens) -> ParseResult<()>;
    fn peek_token(&self, token_type: Tokens) -> bool;
    fn get_priority(&self, token_type: Tokens) -> Priority;
    fn peek_priority(&mut self) -> Priority;
    fn current_priority(&self) -> Priority;
}

pub trait ParserTrait {
    fn parse_program(&mut self) -> Program;
    fn parse_statement(&mut self) -> ParseResult<Statement>;
    fn parse_let_statement(&mut self) -> ParseResult<Statement>;
    fn parse_return_statement(&mut self) -> ParseResult<Statement>;
    fn parse_type_statement(&mut self) -> ParseResult<Statement>;
    fn parse_expression_statement(&mut self) -> ParseResult<Statement>;
    fn parse_expression(&mut self, precedence: Priority) -> ParseResult<Expression>;
    fn parse_block_expression(&mut self) -> ParseResult<BlockExpression>;
    fn parse_array_literal(&mut self) -> ParseResult<ArrayLiteral>;
    fn parse_object_literal(&mut self) -> ParseResult<ObjectLiteral>;
    fn parse_function_literal(&mut self) -> ParseResult<FunctionLiteral>;
}

pub trait TypeParser {
    fn parse_data_type(&mut self) -> ParseResult<DataType>;
    fn parse_data_type_without_next(&mut self) -> ParseResult<DataType>;
    fn parse_function_type(&mut self) -> ParseResult<FunctionType>;
    fn parse_object_type(&mut self) -> ParseResult<ObjectType>;
    fn parse_generic(&mut self) -> ParseResult<Generic>;
    fn parse_generic_identifier(&mut self) -> ParseResult<IdentifierGeneric>;
}

/// **Parses the input string into an AST.**
///
/// for example:
/// ```rust
/// use sntk_core::parser::parser::*;
///
/// let parsed = Parser::from(r#"type X<T, U> = fn(T, U[]) -> object T: U;"#).parse_program();
/// println!("{parsed:#?}");
/// ```
#[derive(Debug)]
pub struct Parser {
    pub lexer: Lexer,
    pub current_token: Token,
    pub peek_token: Token,
    pub position: Position,
    pub errors: Vec<ParsingError>,
}

impl<T> From<T> for Parser
where
    T: Into<String>,
{
    fn from(x: T) -> Self {
        Parser::new(Lexer::new(x))
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            lexer: Lexer::default(),
            current_token: Token::default(),
            peek_token: Token::default(),
            position: Position::default(),
            errors: Vec::new(),
        }
    }
}

impl ParserBase for Parser {
    /// **Creates a new Parser instance.**
    /// it takes an argument of type `Lexer`.
    fn new(lexer: Lexer) -> Self {
        let mut parser = Parser {
            lexer,
            ..Default::default()
        };

        parser.next_token();
        parser.next_token();

        parser
    }

    /// **Advances the current token and the peek token.**
    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();

        self.position = Position::new(self.current_token.position.0, self.current_token.position.1);
    }

    /// **Checks if the current token is of the expected type.**
    fn expect_token(&mut self, token_type: Tokens) -> ParseResult<()> {
        if self.current_token.token_type == token_type {
            self.next_token();

            Ok(())
        } else {
            Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; token_type, self.current_token.token_type },
            )
        }
    }

    /// **Checks if the peek token is of the expected type.**
    fn peek_token(&self, token_type: Tokens) -> bool {
        self.peek_token.token_type == token_type
    }

    /// **Gets the priority from the `Tokens`.**
    ///
    /// * `Lowest`:       0 (`default`)
    /// * `Equals`:       1 (`==`, ...)
    /// * `LessGreater`:  2 (`<`, `>`, ...)
    /// * `Sum`:          3 (`+`, `-`, ...)
    /// * `Product`:      4 (`*`, `/`, ...)
    /// * `Prefix`:       5 (`!expr`, `-expr`, ...)
    /// * `Call`:         6 (`function(...)`)
    /// * `Index`:        7 (`array[index]`)
    fn get_priority(&self, token_type: Tokens) -> Priority {
        match token_type {
            Tokens::Assign => Priority::Equals,
            Tokens::Plus | Tokens::Minus => Priority::Sum,
            Tokens::Slash | Tokens::Asterisk => Priority::Product,
            Tokens::LT | Tokens::GT => Priority::LessGreater,
            Tokens::EQ | Tokens::NEQ => Priority::Equals,
            Tokens::LParen => Priority::Call,
            Tokens::LBracket => Priority::Index,
            _ => Priority::Lowest,
        }
    }

    /// **Gets the priority from the peek token.**
    fn peek_priority(&mut self) -> Priority {
        self.get_priority(self.peek_token.token_type.clone())
    }

    /// **Gets the priority from the current token.**
    fn current_priority(&self) -> Priority {
        self.get_priority(self.current_token.token_type.clone())
    }
}

impl ParserTrait for Parser {
    /// **Parses the input string into an AST.**
    fn parse_program(&mut self) -> Program {
        let mut program = Program::default();

        while self.current_token.token_type != Tokens::EOF {
            match self.parse_statement() {
                Ok(statement) => program.statements.push(statement),
                Err(error) => self.errors.push(error),
            }

            self.next_token();
        }

        program.errors = self.errors.clone();

        if self.errors.len() > 0 {
            program.statements = Vec::new();
        }

        program
    }

    /// **Parses a statement.**
    fn parse_statement(&mut self) -> ParseResult<Statement> {
        match self.current_token.token_type {
            Tokens::Let => self.parse_let_statement(),
            Tokens::Return => self.parse_return_statement(),
            Tokens::Type => self.parse_type_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    /// **Parses a let statement.**
    ///
    /// `let ident: type = expr;`
    fn parse_let_statement(&mut self) -> ParseResult<Statement> {
        self.next_token();

        let ident = Identifier::new(ident! { self }.clone(), position! { self });
        self.next_token();

        self.expect_token(Tokens::Colon)?;

        let data_type = self.parse_data_type()?;

        self.expect_token(Tokens::Assign)?;

        if let Ok(expression) = self.parse_expression(Priority::Lowest) {
            return if self.peek_token(Tokens::Semicolon) {
                self.next_token();

                Ok(Statement::LetStatement(LetStatement::new(
                    data_type,
                    ident,
                    expression,
                    position! { self },
                )))
            } else {
                Err(
                    parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::Semicolon, self.current_token.token_type },
                )
            };
        }

        Err(parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type })
    }

    /// **Parses a return statement.**
    ///
    /// `return expr;`
    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        self.next_token();

        if let Ok(expression) = self.parse_expression(Priority::Lowest) {
            return if self.peek_token(Tokens::Semicolon) {
                self.next_token();

                Ok(Statement::ReturnStatement(ReturnStatement::new(
                    expression,
                    position! { self },
                )))
            } else {
                Err(
                    parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::Semicolon, self.current_token.token_type },
                )
            };
        }

        Err(parsing_error! { self; EXPECTED_EXPRESSION; self.current_token.token_type })
    }

    /// **Parses a type statement.**
    ///
    /// `type <ident><generics?> = <type>;`
    fn parse_type_statement(&mut self) -> ParseResult<Statement> {
        self.next_token();

        let ident = ident! { self };
        self.next_token();

        let generics = if self.current_token.token_type == Tokens::LT {
            self.parse_generic_identifier()?
        } else {
            Vec::new()
        };
        self.next_token();

        self.expect_token(Tokens::Assign)?;

        let data_type = self.parse_data_type()?;

        return if self.current_token.token_type == Tokens::Semicolon {
            Ok(Statement::TypeStatement(TypeStatement::new(
                data_type,
                Identifier::new(ident, position! { self }),
                generics,
                position! { self },
            )))
        } else {
            Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::Semicolon, self.current_token.token_type },
            )
        };
    }

    /// **Parses an expression statement.**
    fn parse_expression_statement(&mut self) -> ParseResult<Statement> {
        let expression = self.parse_expression(Priority::Lowest)?;

        if self.peek_token(Tokens::Semicolon) {
            self.next_token();

            Ok(Statement::ExpressionStatement(ExpressionStatement::new(
                expression,
                position! { self },
            )))
        } else {
            Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::Semicolon, self.current_token.token_type },
            )
        }
    }

    /// **Parses an expression.**
    fn parse_expression(&mut self, priority: Priority) -> ParseResult<Expression> {
        let left_expression = match self.current_token.token_type.clone() {
            Tokens::IDENT(ident) => Some(Ok(Expression::Identifier(Identifier::new(
                ident.clone(),
                position! { self },
            )))),
            Tokens::Number(number) => Some(Ok(Expression::NumberLiteral(NumberLiteral::new(
                number,
                position! { self },
            )))),
            Tokens::String(string) => Some(Ok(Expression::StringLiteral(StringLiteral::new(
                string,
                position! { self },
            )))),
            Tokens::Boolean(boolean) => Some(Ok(Expression::BooleanLiteral(BooleanLiteral::new(
                boolean,
                position! { self },
            )))),
            Tokens::Bang | Tokens::Minus => {
                Some(Ok(Expression::PrefixExpression(PrefixExpression::new(
                    self.current_token.token_type.clone(),
                    Box::new(self.parse_expression(Priority::Prefix)?),
                    position! { self },
                ))))
            }
            Tokens::LParen => {
                self.next_token();

                let expression = self.parse_expression(Priority::Lowest);
                self.next_token();

                if self.current_token.token_type != Tokens::RParen {
                    return Err(
                        parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::RParen, self.current_token.token_type },
                    );
                }

                Some(expression)
            }
            Tokens::LBrace => Some(Ok(Expression::BlockExpression(
                self.parse_block_expression()?,
            ))),
            Tokens::LBracket => Some(Ok(Expression::ArrayLiteral(self.parse_array_literal()?))),
            Tokens::ObjectType => Some(Ok(Expression::ObjectLiteral(self.parse_object_literal()?))),
            Tokens::Function => Some(Ok(Expression::FunctionLiteral(
                self.parse_function_literal()?,
            ))),
            Tokens::If => unimplemented!(),
            _ => None,
        };

        if let None = left_expression {
            if self.current_token.token_type != Tokens::Semicolon {
                return Err(
                    parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type },
                );
            }
        }

        let mut left_expression = left_expression.ok_or_else(
            || parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type },
        )?;

        while !self.peek_token(Tokens::Semicolon) && priority < self.peek_priority() {
            self.next_token();

            left_expression = match self.current_token.token_type {
                Tokens::Plus
                | Tokens::Minus
                | Tokens::Slash
                | Tokens::Asterisk
                | Tokens::Percent
                | Tokens::EQ
                | Tokens::NEQ
                | Tokens::LT
                | Tokens::GT
                | Tokens::LTE
                | Tokens::GTE => Ok(Expression::InfixExpression(InfixExpression::new(
                    Box::new(left_expression?),
                    self.current_token.token_type.clone(),
                    {
                        self.next_token();
                        Box::new(self.parse_expression(self.current_priority())?)
                    },
                    position! { self },
                ))),
                Tokens::LParen => {
                    self.next_token();

                    let mut arguments = Vec::new();

                    if self.current_token.token_type != Tokens::RParen {
                        arguments.push(self.parse_expression(Priority::Lowest)?);

                        while self.current_token.token_type != Tokens::RParen {
                            arguments.push(self.parse_expression(Priority::Lowest)?);
                            self.next_token();

                            if self.current_token.token_type == Tokens::RParen {
                                break;
                            }

                            self.expect_token(Tokens::Comma)?;
                        }

                        if self.current_token.token_type != Tokens::RParen {
                            return Err(
                                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::RParen, self.current_token.token_type },
                            );
                        }
                    }

                    Ok(Expression::CallExpression(CallExpression::new(
                        Box::new(left_expression?),
                        arguments,
                        position! { self },
                    )))
                }
                _ => Err(parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type }),
            };
        }

        left_expression
    }

    /// **Parses a block expression.**
    fn parse_block_expression(&mut self) -> ParseResult<BlockExpression> {
        self.next_token();

        let mut statements = Vec::new();

        while self.current_token.token_type != Tokens::RBrace {
            statements.push(self.parse_statement()?);
            self.next_token();
        }

        Ok(BlockExpression::new(statements, position! { self }))
    }

    /// **Parses an array literal.**
    fn parse_array_literal(&mut self) -> ParseResult<ArrayLiteral> {
        self.next_token();

        let mut elements = Vec::new();

        while self.current_token.token_type != Tokens::RBrace {
            elements.push(self.parse_expression(Priority::Lowest)?);
            self.next_token();

            if self.current_token.token_type == Tokens::RBracket {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        if self.current_token.token_type != Tokens::RBracket {
            return Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::RBracket, self.current_token.token_type },
            );
        }

        Ok(ArrayLiteral::new(elements, position! { self }))
    }

    /// **Parses a object type.**
    fn parse_object_literal(&mut self) -> ParseResult<ObjectLiteral> {
        self.next_token();
        self.next_token();

        let mut elements = Vec::new();

        while self.current_token.token_type != Tokens::RBrace {
            let key = self.parse_expression(Priority::Lowest)?;
            self.next_token();

            self.expect_token(Tokens::Colon)?;

            let value = self.parse_expression(Priority::Lowest)?;
            self.next_token();

            elements.push((key, value));

            if self.current_token.token_type == Tokens::RBrace {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        if self.current_token.token_type != Tokens::RBrace {
            return Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::RBrace, self.current_token.token_type },
            );
        }

        Ok(ObjectLiteral::new(elements, position! { self }))
    }

    /// **Parses a function literal.**
    ///
    /// `fn<Generic>(arg1: Type, arg2: Type) -> Type { ... }`
    fn parse_function_literal(&mut self) -> ParseResult<FunctionLiteral> {
        self.next_token();

        let generics = if self.current_token.token_type == Tokens::LT {
            let result = Some(self.parse_generic_identifier()?);
            self.next_token();

            result
        } else {
            None
        };

        self.expect_token(Tokens::LParen)?;

        let mut parameters = Vec::new();

        while self.current_token.token_type != Tokens::RParen {
            if let Tokens::IDENT(identifier) = self.current_token.token_type.clone() {
                self.next_token();
                self.expect_token(Tokens::Colon)?;

                let data_type = self.parse_data_type()?;

                parameters.push((
                    Identifier::new(identifier.clone(), position! { self }),
                    data_type,
                ));
            } else {
                return Err(
                    parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type },
                );
            }

            if self.current_token.token_type == Tokens::RParen {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        self.expect_token(Tokens::RParen)?;
        self.expect_token(Tokens::Arrow)?;

        let return_type = self.parse_data_type()?;
        if self.current_token.token_type != Tokens::LBrace {
            return Err(
                parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::LBrace, self.current_token.token_type },
            );
        }

        let body = self.parse_block_expression()?;

        Ok(FunctionLiteral::new(
            generics,
            parameters,
            return_type,
            body,
            position! { self },
        ))
    }
}

impl TypeParser for Parser {
    /// **Parses a data type.**
    ///
    /// * `x: number`:   `Number`
    /// * `x: string`:   `String`
    /// * `x: boolean`:  `Boolean`
    /// * `x: T`:        `Identifier("T")`
    /// * `x: T[]`:      `Array(Identifier("T"))`
    /// * `x: T<U, V>`:  `Generic(Identifier("T"), [Identifier("U"), Identifier("V")])`
    /// * `x: fn(number, string) -> boolean`: `Function([Number, String], Boolean)`
    fn parse_data_type(&mut self) -> ParseResult<DataType> {
        let result = self.parse_data_type_without_next();
        self.next_token();
        result
    }

    /// **Parses a data type.**
    ///
    /// extends `parse_data_type` by not advancing the current token.
    fn parse_data_type_without_next(&mut self) -> ParseResult<DataType> {
        let mut data_type = match self.current_token.token_type {
            Tokens::NumberType => Ok(DataType::Number),
            Tokens::StringType => Ok(DataType::String),
            Tokens::BooleanType => Ok(DataType::Boolean),
            Tokens::Function => Ok(DataType::Fn(self.parse_function_type()?)),
            Tokens::ObjectType => Ok(DataType::Object(self.parse_object_type()?)),
            Tokens::IDENT(ref ident) => Ok(DataType::Custom(ident.clone())),
            _ => Err(parsing_error! { self; UNEXPECTED_TOKEN; self.current_token.token_type }),
        };

        if self.peek_token(Tokens::LT) {
            let generics = self.parse_generic()?;

            data_type = Ok(DataType::Generic(generics));
        }

        while self.peek_token(Tokens::LBracket) {
            self.next_token();
            self.next_token();

            if self.current_token.token_type != Tokens::RBracket {
                return Err(
                    parsing_error! { self; EXPECTED_NEXT_TOKEN; Tokens::RBracket, self.current_token.token_type },
                );
            }

            data_type = data_type.map(|t| DataType::Array(Box::new(t)));
        }

        data_type
    }

    /// **Parses a function type.**
    ///
    /// * `fn(number, string) -> boolean`: `Function([Number, String], Boolean)`
    fn parse_function_type(&mut self) -> ParseResult<FunctionType> {
        self.next_token();

        let generics = if self.current_token.token_type == Tokens::LT {
            let result = Some(self.parse_generic_identifier()?);
            self.next_token();

            result
        } else {
            None
        };

        self.expect_token(Tokens::LParen)?;

        let mut parameters = Vec::new();

        while self.current_token.token_type != Tokens::RParen {
            parameters.push(self.parse_data_type()?);

            if self.current_token.token_type == Tokens::RParen {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        self.expect_token(Tokens::RParen)?;
        self.expect_token(Tokens::Arrow)?;

        let return_type = self.parse_data_type_without_next()?;

        Ok(FunctionType::new(generics, parameters, return_type))
    }

    /// **Parses a object type.**
    ///
    /// `object string: number` -> `Object(String, Number)`
    fn parse_object_type(&mut self) -> ParseResult<ObjectType> {
        self.next_token();

        let key_type = self.parse_data_type()?;
        self.expect_token(Tokens::Colon)?;
        let value_type = self.parse_data_type_without_next()?;

        Ok(ObjectType::new(key_type, value_type))
    }

    /// **Parses a generic type.**
    ///
    /// `T<U[], V>`: `Generic(Identifier("T"), [Array(Identifier("U")), Identifier("V")])`
    fn parse_generic(&mut self) -> ParseResult<Generic> {
        let ident = ident! { self };
        self.next_token();

        let mut generics = Vec::new();

        self.expect_token(Tokens::LT)?;

        while self.current_token.token_type != Tokens::GT {
            let data_type = self.parse_data_type()?;

            generics.push(data_type);

            if self.current_token.token_type == Tokens::GT {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        Ok(Generic::new(DataType::Custom(ident), generics))
    }

    /// **Parses a generic type.**
    ///
    /// `T<U, V>`: `Generic(Identifier("T"), [Identifier("U"), Identifier("V")])`
    fn parse_generic_identifier(&mut self) -> ParseResult<IdentifierGeneric> {
        let mut generics = Vec::new();

        self.expect_token(Tokens::LT)?;

        while self.current_token.token_type != Tokens::GT {
            let ident = ident! { self };
            self.next_token();

            generics.push(Identifier::new(ident, position! { self }));

            if self.current_token.token_type == Tokens::GT {
                break;
            }

            self.expect_token(Tokens::Comma)?;
        }

        Ok(generics)
    }
}
