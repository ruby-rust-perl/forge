use std::rc::Rc;
use super::{
    ParseError,
    ParseResult,
    SrcRef,
};

#[derive(Debug)]
pub struct Node<T>(pub T, pub SrcRef);

#[derive(Debug)]
pub enum Expr {
    None,
    LiteralNumber(f64),
    LiteralString(String),
    LiteralChar(char),
    LiteralBoolean(bool),
    LiteralNull,
    Ident(Node<String>),
    List(Node<Vec<Node<Expr>>>),
    Map(Node<Vec<(Node<Expr>, Node<Expr>)>>),

    Call(SrcRef, Box<Node<Expr>>, Node<Vec<Node<Expr>>>),
    DotAccess(SrcRef, Box<Node<Expr>>, Node<String>),
    Index(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),

    UnaryNot(SrcRef, Box<Node<Expr>>),
    UnaryNeg(SrcRef, Box<Node<Expr>>),
    UnaryInput(SrcRef, Box<Node<Expr>>),
    UnaryClone(SrcRef, Box<Node<Expr>>),
    UnaryMirror(SrcRef, Box<Node<Expr>>),

    BinaryMul(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryDiv(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryRem(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryAdd(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinarySub(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryGreater(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryGreaterEq(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryLess(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryLessEq(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryEq(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryNotEq(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryAnd(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryOr(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryXor(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryRange(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),
    BinaryAs(SrcRef, Box<Node<Expr>>, Box<Node<Expr>>),

    BinaryAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),
    BinaryAddAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),
    BinarySubAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),
    BinaryMulAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),
    BinaryDivAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),
    BinaryRemAssign(SrcRef, Node<LVal>, Box<Node<Expr>>),

    Fn(Rc<String>, Rc<(Node<Args>, Node<Block>)>),
}

#[derive(Debug)]
pub enum LVal {
    Local(Node<String>),
    Index(Box<Node<Expr>>, Box<Node<Expr>>),
}

#[derive(Debug)]
pub struct Args(pub Vec<Node<String>>);

#[derive(Debug)]
pub struct Block(pub Vec<Node<Stmt>>);

#[derive(Debug)]
pub enum Stmt {
    Expr(Node<Expr>),
    Print(Node<Expr>),
    If(Node<Expr>, Node<Block>),
    IfElse(Node<Expr>, Node<Block>, Node<Block>),
    While(Node<Expr>, Node<Block>),
    For(Node<String>, Node<Expr>, Node<Block>),
    Decl(Node<String>, Node<Expr>),
    Return(Node<Expr>),
}

// Utility

struct Spaces(usize);

impl std::fmt::Display for Spaces {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for _ in 0..self.0 * 2 {
            let _ = write!(f, " ");
        }
        Ok(())
    }
}

impl Node<Expr> {
    pub fn into_lvalue(self, r: SrcRef) -> ParseResult<Node<LVal>> {
        match self {
            Node(Expr::Ident(ident), r) => Ok(Node(LVal::Local(ident), r)),
            Node(Expr::Index(_, expr, index), r) => Ok(Node(LVal::Index(expr, index), r)),
            Node(_, _) => Err(ParseError::At(r, Box::new(ParseError::NotAnLValue))),
        }
    }
}

impl Expr {
    pub fn print_debug(&self, depth: usize) {
        match self {
            Expr::None => println!("{}None expression", Spaces(depth)),
            Expr::LiteralNumber(x) => println!("{}Number literal '{}'", Spaces(depth), x),
            Expr::LiteralString(s) => println!("{}String literal '{}'", Spaces(depth), s),
            Expr::LiteralChar(c) => println!("{}Character literal '{}'", Spaces(depth), c),
            Expr::LiteralBoolean(b) => println!("{}Boolean literal '{}'", Spaces(depth), b),
            Expr::LiteralNull => println!("{}Null literal", Spaces(depth)),
            Expr::Ident(s) => println!("{}Identifier '{}'", Spaces(depth), s.0),
            Expr::List(items) => {
                println!("{}List", Spaces(depth));
                for item in &items.0 {
                    println!("{}Item", Spaces(depth + 1));
                    item.0.print_debug(depth + 2);
                }
            },
            Expr::Map(items) => {
                println!("{}List", Spaces(depth));
                for (key, val) in &items.0 {
                    println!("{}Key", Spaces(depth + 1));
                    key.0.print_debug(depth + 2);
                    println!("{}Value", Spaces(depth + 1));
                    val.0.print_debug(depth + 2);
                }
            },
            Expr::Call(_, expr, params) => {
                println!("{}Call", Spaces(depth));
                expr.0.print_debug(depth + 1);
                for param in &params.0 {
                    println!("{}Parameter", Spaces(depth + 1));
                    param.0.print_debug(depth + 1);
                }
            },
            Expr::DotAccess(_, expr, s) => {
                println!("{}Dot access '{}'", Spaces(depth), s.0);
                expr.0.print_debug(depth + 1);
            },
            Expr::Index(_, expr, index) => {
                println!("{}Index access", Spaces(depth));
                expr.0.print_debug(depth + 1);
                index.0.print_debug(depth + 1);
            },
            Expr::UnaryNot(_, expr) => {
                println!("{}Unary not", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Expr::UnaryNeg(_, expr) => {
                println!("{}Unary neg", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Expr::UnaryInput(_, expr) => {
                println!("{}Unary input", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Expr::UnaryClone(_, expr) => {
                println!("{}Unary clone", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Expr::UnaryMirror(_, expr) => {
                println!("{}Unary mirror", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Expr::BinaryMul(_, left, right) => {
                println!("{}Binary mul", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryDiv(_, left, right) => {
                println!("{}Binary div", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryRem(_, left, right) => {
                println!("{}Binary rem", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryAdd(_, left, right) => {
                println!("{}Binary add", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinarySub(_, left, right) => {
                println!("{}Binary sub", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryGreater(_, left, right) => {
                println!("{}Binary greater", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryGreaterEq(_, left, right) => {
                println!("{}Binary greater_eq", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryLess(_, left, right) => {
                println!("{}Binary less", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryLessEq(_, left, right) => {
                println!("{}Binary less_eq", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryEq(_, left, right) => {
                println!("{}Binary eq", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryNotEq(_, left, right) => {
                println!("{}Binary eq", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryAnd(_, left, right) => {
                println!("{}Binary and", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryOr(_, left, right) => {
                println!("{}Binary or", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryXor(_, left, right) => {
                println!("{}Binary xor", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryRange(_, left, right) => {
                println!("{}Binary range", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryAs(_, left, right) => {
                println!("{}Binary as", Spaces(depth));
                left.0.print_debug(depth + 1);
                right.0.print_debug(depth + 1);
            },
            Expr::BinaryAssign(_, target, expr) => {
                println!("{}Binary assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::BinaryAddAssign(_, target, expr) => {
                println!("{}Binary add-assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::BinarySubAssign(_, target, expr) => {
                println!("{}Binary sub-assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::BinaryMulAssign(_, target, expr) => {
                println!("{}Binary add-assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::BinaryDivAssign(_, target, expr) => {
                println!("{}Binary div-assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::BinaryRemAssign(_, target, expr) => {
                println!("{}Binary rem-assign", Spaces(depth));
                target.0.print_debug(depth + 1);
                expr.0.print_debug(depth + 1);
            },
            Expr::Fn(_, rc) => {
                println!("{}Function", Spaces(depth));
                (rc.0).0.print_debug(depth + 1);
                (rc.1).0.print_debug(depth + 1);
            },
        }
    }
}

impl LVal {
    pub fn print_debug(&self, depth: usize) {
        match self {
            LVal::Local(i) => println!("{}Local l-value '{}'", Spaces(depth), i.0),
            LVal::Index(expr, index) => {
                println!("{}Indexed l-value", Spaces(depth));
                expr.0.print_debug(depth + 1);
                index.0.print_debug(depth + 1);
            },
        }
    }
}

impl Stmt {
    pub fn print_debug(&self, depth: usize) {
        match self {
            Stmt::Expr(expr) => {
                println!("{}Expression statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Stmt::Print(expr) => {
                println!("{}Print statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
            Stmt::If(expr, block) => {
                println!("{}If statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
                block.0.print_debug(depth + 1);
            },
            Stmt::IfElse(expr, true_block, false_block) => {
                println!("{}If-else statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
                true_block.0.print_debug(depth + 1);
                false_block.0.print_debug(depth + 1);
            },
            Stmt::While(expr, block) => {
                println!("{}While statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
                block.0.print_debug(depth + 1);
            },
            Stmt::For(ident, expr, block) => {
                println!("{}For statement '{}'", Spaces(depth), ident.0);
                expr.0.print_debug(depth + 1);
                block.0.print_debug(depth + 1);
            },
            Stmt::Decl(ident, expr) => {
                println!("{}Declaration statement '{}'", Spaces(depth), ident.0);
                expr.0.print_debug(depth + 1);
            },
            Stmt::Return(expr) => {
                println!("{}Return statement", Spaces(depth));
                expr.0.print_debug(depth + 1);
            },
        }
    }
}

impl Block {
    pub fn print_debug(&self, depth: usize) {
        println!("{}Block", Spaces(depth));
        for stmt in &self.0 {
            stmt.0.print_debug(depth + 2);
        }
    }
}

impl Args {
    pub fn print_debug(&self, depth: usize) {
        println!("{}Args", Spaces(depth));
        for arg in &self.0 {
            println!("{}Argument '{}'", Spaces(depth + 2), arg.0);
        }
    }
}
