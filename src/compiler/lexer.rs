pub use logos::*;

#[derive(Debug, Clone, PartialEq, Logos)]
#[logos(skip r"[\s]")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[regex(r"([\d_]+|0x[\da-fA-F_]+|0b[01_]+)", callback = parse_int)]
    Integer(u128),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", priority = 0)]
    Ident,

    #[token(";")]
    Semicolon,

    #[token("(")]
    RoBracketS,
    #[token(")")]
    RoBracketE,

    #[token("[")]
    SqBracketS,
    #[token("]")]
    SqBracketE,

    #[token("{")]
    CuBracketS,
    #[token("}")]
    CuBracketE,

    #[regex(r"(\+|\-|\*|/|%|&|\||\^|<<|>>)(=)?", callback = parse_operator)]
    #[regex(r"(<|>|!|==|!=|<=|>=|&&|\|\||=|\.)", callback = parse_operator)]
    Operator(Operator),

    #[token("var")]
    Var,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("return")]
    Return,

    #[token(",")]
    Comma,

    #[regex(r"[\n]+", priority = 10)]
    NewLine,

    None
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add, Sub, Mlt, Div, Mod,
    Assign, OpAssign(Box<Operator>),

    LBrack,
    Eq, NE, GT, GE, LT, LE,
    BAnd, BOr, BXOr, Not,
    LAnd, LOr,

    LSh, RSh,
    FnCall(String),
    Of,
}

impl Operator {
    pub fn percedence(&self, unary: bool) -> usize {
        use Operator::*;
        if !unary {
            match self {
                LBrack | FnCall(_) | Of
                    => 13,
                Mlt | Div | Mod
                    => 11,
                Add | Sub
                    => 10,
                LSh | RSh
                    => 9,
                LT | LE | GT | GE
                    => 8,
                Eq | NE
                    => 7,
                BAnd
                    => 6,
                BXOr
                    => 5,
                BOr
                    => 4,
                LAnd
                    => 3,
                LOr
                    => 2,
                Assign | OpAssign(_)
                    => 1,
                _ => unreachable!()
            }
        } else {
            match self {
                Add | Sub | Mlt | Not
                    => 12,
                _ => unreachable!(),
            }
        }
    }
    pub fn is_left(&self) -> bool {
        use Operator::*;
        match self {
            Assign | OpAssign(_)
                => false,
            _ => true,
        }
    }
}

fn parse_int(lex: &mut Lexer<Token>) -> u128 {
    let s = lex.slice().replace("_", "");
    match s.chars().nth(1).unwrap_or(' ') {
        'x' => u128::from_str_radix(&s[2..], 16),
        'b' => u128::from_str_radix(&s[2..], 2),
        _   => s.parse(),
    }.unwrap()
}

fn parse_operator(lex: &mut Lexer<Token>) -> Operator {
    let s = lex.slice();

    use Operator::*;
    match s {
        "==" => Eq,
        "!=" => NE,
        "<"  => LT,
        "<=" => LE,
        ">"  => GT,
        ">=" => GE,
        "!"  => Not,
        "&&" => LAnd,
        "||" => LOr,
        "."  => Of,

        _ => if s.chars().last().unwrap() == '=' && s.len() > 1 {
            Operator::OpAssign(Box::new(_parse_oper(&s[0..s.len()-1])))
        } else {
            _parse_oper(s)
        }
    }
}

fn _parse_oper(s: &str) -> Operator {
    use Operator::*;
    match s {
        "+" => Add, "-" => Sub,
        "*" => Mlt, "/" => Div,
        "%" => Mod, "=" => Assign,
        "|" => BOr, "&" => BAnd,
        "^" => BXOr,
        "<<" => LSh, ">>" => RSh,
        _ => unreachable!()
    }
}
