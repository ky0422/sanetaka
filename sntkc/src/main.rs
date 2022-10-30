use sntk_compiler::compiler::{Compiler, CompilerTrait};
use sntk_core::parser::parser::{Parser, ParserTrait};
use sntk_ir::interpreter::InterpreterBase;

fn main() {
    let mut compiler = Compiler::new(
        Parser::from(
            r#"
let x: boolean[] = [true, false];
print(x);
            "#
            .to_string(),
        )
        .parse_program(),
    );

    match &mut compiler.compile_program() {
        Ok(interpreter) => interpreter.run(),
        Err(error) => println!("{error}"),
    }
}
