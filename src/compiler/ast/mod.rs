use super::*;

pub mod ast;
pub use ast::*;
pub mod node;
pub use node::*;

pub type ANode = (Node, Span);
pub type AExpr = (Expr, Span);
pub type AType = (Type, Span);
pub type AString = (String, Span);
