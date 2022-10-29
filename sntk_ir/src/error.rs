#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct IrRuntime {
    pub message: String,
    pub pointer: Option<usize>,
}

impl IrRuntime {
    pub fn new(message: &str, args: Vec<String>, pointer: Option<usize>) -> Self {
        let mut message = message.to_string();

        args.iter().enumerate().for_each(|(i, arg)| {
            message = message.replace(&format!("{{{i}}}"), arg);
        });

        IrRuntime { message, pointer }
    }
}

impl std::fmt::Display for IrRuntime {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{} {}", self.message, match self.pointer {
            Some(p) => format!("(at {})", p),
            None => "".to_string()
        })
    }
}

macro_rules! messages {
    ($( $name:ident => $message:expr );*;) => {
        $( pub const $name: &str = $message; )*
    };
}

messages! {
    NOT_DEFINED => "Name '{0}' is not defined";
    NOT_A_FUNCTION => "Not a function: {0}";
    NOT_A_BOOLEAN => "Not a boolean: {0}";
    NOT_A_LITERAL_VALUE => "Not a literal value: {0}";
    INVALID_OPERAND => "Invalid operand: {0}";
    INVAILD_ARGUMENTS => "Invalid arguments: {0}";
    POP_EMPTY_STACK => "Pop empty stack";
}

#[macro_export]
macro_rules! runtime_error {
    ($self:ident; $msg:ident; $( $r:expr ),*) => {
        panic!("{}", IrRuntime::new($msg, vec![$( $r.to_string() ),*], Some($self.instruction_pointer)))
    };
    (@stack $self:ident; $msg:ident; $( $r:expr ),*) => {
        panic!("{}", IrRuntime::new($msg, vec![$( $r.to_string() ),*], None))
    };
}
