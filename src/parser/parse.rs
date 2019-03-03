use std::{
    slice,
    fmt,
    rc::Rc,
};
use super::{
    ParseError,
    ParseResult,
    SrcRef,
    Token,
    Lexeme,
    ast::{
        Node,
        Expr,
        LVal,
        Stmt,
        Block,
        Args,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Lexeme(Lexeme),
    Ident,
    Primary,
    Stmt,
    Assignment,
    LVal,
    End,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Item::Lexeme(lexeme) => match lexeme {
                Lexeme::Ident(ident) => write!(f, "identifier '{}'", ident),
                lexeme => write!(f, "'{}'", lexeme),
            },
            Item::Ident => write!(f, "identifier"),
            Item::Primary => write!(f, "primary expression"),
            Item::Stmt => write!(f, "statement"),
            Item::Assignment => write!(f, "assignment"),
            Item::LVal => write!(f, "l-value"),
            Item::End => write!(f, "end of input"),
        }
    }
}

fn expected(expected: Item, found: Item, src_ref: SrcRef) -> ParseError {
    ParseError::At(
        src_ref,
        Box::new(ParseError::Expected(expected, found)),
    )
}

#[derive(Clone)]
pub struct ParseCtx<'a> {
    tokens: slice::Iter<'a, Token>,
    code: Rc<String>,
}

impl<'a> ParseCtx<'a> {
    pub fn new(tokens: slice::Iter<'a, Token>, code: Rc<String>) -> Self {
        Self {
            tokens,
            code,
        }
    }

    #[allow(dead_code)]
    pub fn src_ref(&self) -> SrcRef {
        self.tokens
            .clone()
            .next()
            .map(|t| t.1.clone())
            .unwrap_or(SrcRef::end())
    }

    fn advance(&mut self) {
        self.tokens.next();
    }

    fn peek(&self) -> Token {
        self.tokens.clone().next().unwrap_or(&Token(Lexeme::Eof, SrcRef::end())).clone()
    }

    fn read_ident(&mut self) -> ParseResult<Node<String>> {
        match self.peek() {
            Token(Lexeme::Ident(s), r) => {
                self.advance();
                Ok(Node(s, r))
            },
            Token(l, r) => Err(expected(Item::Ident, Item::Lexeme(l), r)),
        }
    }

    fn read_primary(&mut self) -> ParseResult<(Node<Expr>, Option<ParseError>)> {
        let expr = match self.peek() {
            Token(Lexeme::Number(x), r) => Node(Expr::LiteralNumber(x), r),
            Token(Lexeme::String(s), r) => Node(Expr::LiteralString(s), r),
            Token(Lexeme::Char(c), r) => Node(Expr::LiteralChar(c), r),
            Token(Lexeme::True, r) => Node(Expr::LiteralBoolean(true), r),
            Token(Lexeme::False, r) => Node(Expr::LiteralBoolean(false), r),
            Token(Lexeme::Null, r) => Node(Expr::LiteralNull, r),
            Token(Lexeme::Ident(s), r) => Node(Expr::Ident(Node(s, r)), r),
            Token(Lexeme::LParen, _r) => {
                let mut this = self.clone();
                let (paren_expr, err) = this.read_paren_expr()?;
                *self = this;
                return Ok((paren_expr, Some(err)));
            },
            Token(Lexeme::Pipe, _r) => {
                let mut this = self.clone();
                let (fn_expr, err) = this.read_fn_expr()?;
                *self = this;
                return Ok((fn_expr, Some(err)));
            },
            Token(Lexeme::LBrack, _r) => {
                // Try reading list first
                let mut this = self.clone();
                let max_err = match this.read_list_expr() {
                    Ok((expr, err)) => {
                        *self = this;
                        return Ok((expr, Some(err)));
                    },
                    Err(err) => err,
                };

                // Then list clone
                let mut this = self.clone();
                let max_err = match this.read_list_clone_expr() {
                    Ok((expr, err)) => {
                        *self = this;
                        return Ok((expr, Some(err)));
                    },
                    Err(err) => err.max(max_err),
                };
                // Then a map
                let mut this = self.clone();
                let (map_expr, err) = this.read_map_expr().map_err(|err| err.max(max_err.clone()))?;
                *self = this;
                return Ok((map_expr, Some(err.max(max_err))));
            },
            Token(l, r) => return Err(expected(Item::Primary, Item::Lexeme(l), r)),
        };
        self.advance();
        Ok((expr, None))
    }

    fn read_access(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, err) = self.read_primary()?;

        let mut max_err = err.unwrap_or(ParseError::phoney());

        loop {
            let mut this = self.clone();
            match this.read_member() {
                Ok((dot_r, Node(ident, r), err)) => {
                    *self = this;
                    let r_union = expr.1.union(&r).union(&dot_r);
                    expr = Node(Expr::DotAccess(dot_r, Box::new(expr), Node(ident, r)), r_union);
                    max_err = err.max(max_err);
                    continue;
                },
                Err(err) => max_err = err.max(max_err),
            }

            let mut this = self.clone();
            match this.read_index() {
                Ok((dot_r, index_expr, err)) => {
                    *self = this;
                    let r_union = expr.1.union(&index_expr.1).union(&dot_r);
                    expr = Node(Expr::Index(dot_r, Box::new(expr), Box::new(index_expr)), r_union);
                    max_err = err.max(max_err);
                    continue;
                },
                Err(err) => max_err = err.max(max_err),
            }

            return Ok((expr, max_err));
        }
    }

    fn read_call(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_access()?;

        loop {
            let mut this = self.clone();
            match this.read_params() {
                Ok((Node(params, params_r), err)) => {
                    *self = this;
                    let r_union = params
                        .iter()
                        .fold(SrcRef::empty(), |r, p| p.1.union(&r));
                    expr = Node(Expr::Call(params_r, Box::new(expr), Node(params, params_r)), r_union);
                    max_err = err.max(max_err);
                },
                Err(err) => return Ok((expr, err.max(max_err))),
            };
        }
    }

    fn read_as(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_call()?;

        loop {
            match self.peek() {
                Token(Lexeme::As, r) => {
                    self.advance();
                    let (operand, err) = self.read_call()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryAs(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_unary(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        Ok(match self.peek() {
            Token(Lexeme::Bang, r) => {
                self.advance();
                let (operand, err) = self.read_as()?;
                let r_union = r.union(&operand.1);
                (Node(Expr::UnaryNot(r, Box::new(operand)), r_union), err)
            },
            Token(Lexeme::Minus, r) => {
                self.advance();
                let (operand, err) = self.read_as()?;
                let r_union = r.union(&operand.1);
                (Node(Expr::UnaryNeg(r, Box::new(operand)), r_union), err)
            },
            _ => self.read_as()?,
        })
    }

    fn read_multiplication(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_unary()?;

        loop {
            match self.peek() {
                Token(Lexeme::Star, r) => {
                    self.advance();
                    let (operand, err) = self.read_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryMul(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Slash, r) => {
                    self.advance();
                    let (operand, err) = self.read_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryDiv(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Percent, r) => {
                    self.advance();
                    let (operand, err) = self.read_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryRem(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_addition(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_multiplication()?;

        loop {
            match self.peek() {
                Token(Lexeme::Plus, r) => {
                    self.advance();
                    let (operand, err) = self.read_multiplication()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryAdd(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Minus, r) => {
                    self.advance();
                    let (operand, err) = self.read_multiplication()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinarySub(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_high_binary(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_addition()?;

        loop {
            match self.peek() {
                Token(Lexeme::DotDot, r) => {
                    self.advance();
                    let (operand, err) = self.read_addition()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryRange(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_mid_unary(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        Ok(match self.peek() {
            Token(Lexeme::Input, r) => {
                self.advance();
                let (operand, err) = self.read_mid_unary()?;
                let r_union = r.union(&operand.1);
                (Node(Expr::UnaryInput(r, Box::new(operand)), r_union), err)
            },
            Token(Lexeme::Clone, r) => {
                self.advance();
                let (operand, err) = self.read_mid_unary()?;
                let r_union = r.union(&operand.1);
                (Node(Expr::UnaryClone(r, Box::new(operand)), r_union), err)
            },
            Token(Lexeme::Mirror, r) => {
                self.advance();
                let (operand, err) = self.read_mid_unary()?;
                let r_union = r.union(&operand.1);
                (Node(Expr::UnaryMirror(r, Box::new(operand)), r_union), err)
            },
            _ => self.read_high_binary()?,
        })
    }

    fn read_comparison(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_mid_unary()?;

        loop {
            match self.peek() {
                Token(Lexeme::Greater, r) => {
                    self.advance();
                    let (operand, err) = self.read_mid_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryGreater(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::GreaterEq, r) => {
                    self.advance();
                    let (operand, err) = self.read_mid_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryGreaterEq(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Less, r) => {
                    self.advance();
                    let (operand, err) = self.read_mid_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryLess(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::LessEq, r) => {
                    self.advance();
                    let (operand, err) = self.read_mid_unary()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryLessEq(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_equivalence(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_comparison()?;

        loop {
            match self.peek() {
                Token(Lexeme::Eq, r) => {
                    self.advance();
                    let (operand, err) = self.read_comparison()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryEq(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::BangEq, r) => {
                    self.advance();
                    let (operand, err) = self.read_comparison()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryNotEq(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_logical(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let (mut expr, mut max_err) = self.read_equivalence()?;

        loop {
            match self.peek() {
                Token(Lexeme::And, r) => {
                    self.advance();
                    let (operand, err) = self.read_equivalence()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryAnd(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Or, r) => {
                    self.advance();
                    let (operand, err) = self.read_equivalence()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryOr(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(Lexeme::Xor, r) => {
                    self.advance();
                    let (operand, err) = self.read_equivalence()?;
                    let r_union = r.union(&expr.1).union(&operand.1);
                    expr = Node(Expr::BinaryXor(r, Box::new(expr), Box::new(operand)), r_union);
                    max_err = err.max(max_err);
                },
                Token(_l, _r) => return Ok((expr, max_err)),
            };
        }
    }

    fn read_assignment(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let mut this = self.clone();
        let (Node(expr, expr_r), max_err) = match this.read_logical() {
            Ok((expr, err)) => {
                *self = this;
                (expr, err)
            },
            Err(err) => return Err(err),
        };

        match self.peek() {
            Token(Lexeme::Assign, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinaryAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(Lexeme::PlusEq, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinaryAddAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(Lexeme::MinusEq, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinarySubAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(Lexeme::StarEq, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinaryMulAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(Lexeme::SlashEq, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinaryDivAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(Lexeme::PercentEq, r) => {
                self.advance();
                let lvalue = Node(expr, expr_r).into_lvalue(r.union(&expr_r)).map_err(|err| err.max(max_err.clone()))?;
                let (operand, err) = self.read_logical()?;
                let r_union = r.union(&expr_r).union(&operand.1);
                Ok((Node(Expr::BinaryRemAssign(r, lvalue, Box::new(operand)), r_union), err.max(max_err)))
            },
            Token(l, r) => Ok((Node(expr, expr_r), expected(Item::Assignment, Item::Lexeme(l), r).max(max_err))),
        }
    }

    fn read_lvalue(&mut self) -> ParseResult<(Node<LVal>, ParseError)> {
        const ELEMENT: &'static str = "lvalue";

        let mut this = self.clone();
        let max_err = match this.read_index() {
            Ok((r, index, err)) => {
                *self = this;
                unimplemented!();
                //return Ok((index, err))
            },
            Err(err) => err,
        };

        let mut this = self.clone();
        let max_err = match this.read_member() {
            Ok((r, member, err)) => {
                *self = this;
                unimplemented!();
                //return Ok((member, err))
            },
            Err(err) => err.max(max_err),
        };

        let max_err = match self.read_ident() {
            Ok(ident) => {
                return Ok((Node(LVal::Local(Node(ident.0, ident.1)), ident.1), ParseError::Phoney))
            },
            Err(err) => err.while_parsing(ELEMENT).max(max_err),
        };

        let next = self.peek();
        Err(expected(Item::LVal, Item::Lexeme(next.0), next.1).max(max_err))
    }

    fn read_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        const ELEMENT: &'static str = "expression";

        let mut this = self.clone();
        match this.read_assignment() {
            Ok((expr, err)) => {
                *self = this;
                Ok((expr, err))
            },
            Err(err) => self.read_logical().map_err(|err| err.while_parsing(ELEMENT)).map_err(|e| e.max(err)),
        }
    }

    fn read_paren_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        let r_start = match self.peek() {
            Token(Lexeme::LParen, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LParen), Item::Lexeme(l), r)),
        };

        let (expr, max_err) = self.read_expr()?;

        match self.peek() {
            Token(Lexeme::RParen, r) => {
                self.advance();
                let r_union = expr.1.union(&r_start).union(&r);
                Ok((Node(expr.0, r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RParen), Item::Lexeme(l), r).max(max_err)),
        }
    }

    fn read_member(&mut self) -> ParseResult<(SrcRef, Node<String>, ParseError)> {
        let dot_r = match self.peek() {
            Token(Lexeme::Dot, r) => { self.advance(); r},
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Dot), Item::Lexeme(l), r))
        };
        Ok((dot_r, self.read_ident()?, ParseError::Phoney))
    }

    fn read_index(&mut self) -> ParseResult<(SrcRef, Node<Expr>, ParseError)> {
        let r_start = match self.peek() {
            Token(Lexeme::LBrack, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LBrack), Item::Lexeme(l), r)),
        };

        let (expr, max_err) = self.read_expr()?;

        match self.peek() {
            Token(Lexeme::RBrack, r) => {
                self.advance();
                let r_union = expr.1.union(&r_start).union(&r);
                Ok((r_union, expr, max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RBrack), Item::Lexeme(l), r).max(max_err)),
        }
    }

    fn read_fn_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        const ELEMENT: &'static str = "function";

        let r_start = match self.peek() {
            Token(Lexeme::Pipe, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Pipe), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (args, max_err) = self.read_args().map_err(|err| err.while_parsing(ELEMENT))?;

        let r_middle = match self.peek() {
            Token(Lexeme::Pipe, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Pipe), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        };

        let (block, max_err) = self.read_block().map_err(|err| err.max(max_err).while_parsing(ELEMENT))?;

        let r_union = args.1.union(&r_start).union(&r_middle).union(&block.1);
        Ok((Node(Expr::Fn(self.code.clone(), Rc::new((Node(args.0, args.1.union(&r_start).union(&r_middle)), block))), r_union), max_err.while_parsing(ELEMENT)))
    }

    fn read_list_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        const ELEMENT: &'static str = "list";

        let r_start = match self.peek() {
            Token(Lexeme::LBrack, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LBrack), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (items, max_err) = self.read_paramlist().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::RBrack, r) => {
                self.advance();
                let r_union = items.1.union(&r_start).union(&r);
                Ok((Node(Expr::List(items), r_union), max_err.while_parsing(ELEMENT)))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RBrack), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_list_clone_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        const ELEMENT: &'static str = "list";

        let r_start = match self.peek() {
            Token(Lexeme::LBrack, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LBrack), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (item, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        let r_middle = match self.peek() {
            Token(Lexeme::Semicolon, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Semicolon), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        };

        let (num, max_err) = self.read_expr().map_err(|err| err.max(max_err).while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::RBrack, r) => {
                self.advance();
                let r_union = item.1.union(&num.1).union(&r_start).union(&r_middle).union(&r);
                Ok((Node(Expr::ListClone(Box::new(item), Box::new(num)), r_union), max_err.while_parsing(ELEMENT)))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RBrack), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_paramlist(&mut self) -> ParseResult<(Node<Vec<Node<Expr>>>, ParseError)> {
        let mut params = vec![];
        let mut r_total = SrcRef::empty();
        let mut max_err = ParseError::Phoney;

        loop {
            let mut this = self.clone();
            match this.read_expr() {
                Ok((expr, err)) => {
                    *self = this;
                    r_total = r_total.union(&expr.1);
                    params.push(expr);
                    max_err = err.max(max_err);
                },
                Err(err) => {
                    max_err = err.max(max_err);
                    break;
                },
            }

            match self.peek() {
                Token(Lexeme::Comma, r) => {
                    self.advance();
                    r_total = r_total.union(&r);
                },
                Token(l, r) => {
                    max_err = expected(Item::Lexeme(Lexeme::Comma), Item::Lexeme(l), r).max(max_err);
                    break;
                },
            }
        }

        Ok((Node(params, r_total), max_err))
    }

    fn read_params(&mut self) -> ParseResult<(Node<Vec<Node<Expr>>>, ParseError)> {
        let r_start = match self.peek() {
            Token(Lexeme::LParen, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LParen), Item::Lexeme(l), r)),
        };

        let (params, max_err) = self.read_paramlist()?;

        match self.peek() {
            Token(Lexeme::RParen, r) => {
                self.advance();
                let r_union = params.1.union(&r_start).union(&r);
                Ok((Node(params.0, r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RParen), Item::Lexeme(l), r).max(max_err)),
        }
    }

    fn read_map_expr(&mut self) -> ParseResult<(Node<Expr>, ParseError)> {
        const ELEMENT: &'static str = "map";

        let r_start = match self.peek() {
            Token(Lexeme::LBrack, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LBrack), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (items, max_err) = self.read_maplist().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::RBrack, r) => {
                self.advance();
                let r_union = items.1.union(&r_start).union(&r);
                Ok((Node(Expr::Map(items), r_union), max_err.while_parsing(ELEMENT)))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RBrack), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_maplist(&mut self) -> ParseResult<(Node<Vec<(Node<Expr>, Node<Expr>)>>, ParseError)> {
        let mut maps = vec![];
        let mut r_total = SrcRef::empty();
        let mut max_err = ParseError::Phoney;

        loop {
            let mut this = self.clone();
            let key = match this.read_expr() {
                Ok((expr, err)) => {
                    r_total = r_total.union(&expr.1);
                    max_err = err.max(max_err);
                    expr
                },
                Err(err) => {
                    max_err = err.max(max_err);
                    break;
                },
            };

            match this.peek() {
                Token(Lexeme::Colon, r) => {
                    this.advance();
                    r_total = r_total.union(&r);
                },
                Token(l, r) => {
                    max_err = expected(Item::Lexeme(Lexeme::Colon), Item::Lexeme(l), r).max(max_err);
                    break;
                },
            }

            let val = match this.read_expr() {
                Ok((expr, err)) => {
                    r_total = r_total.union(&expr.1);
                    max_err = err.max(max_err);
                    expr
                },
                Err(err) => {
                    max_err = err.max(max_err);
                    break;
                },
            };

            *self = this;
            maps.push((key, val));

            match self.peek() {
                Token(Lexeme::Comma, r) => {
                    self.advance();
                    r_total = r_total.union(&r);
                },
                Token(l, r) => {
                    max_err = expected(Item::Lexeme(Lexeme::Comma), Item::Lexeme(l), r).max(max_err);
                    break;
                },
            }
        }

        Ok((Node(maps, r_total), max_err))
    }

    fn read_expr_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "expression statement";

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::Semicolon, r) => {
                self.advance();
                let r_union = expr.1.union(&r);
                Ok((Node(Stmt::Expr(expr), r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::Semicolon), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_print_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "print statement";

        let r_start = match self.peek() {
            Token(Lexeme::Print, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Print), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::Semicolon, r) => {
                self.advance();
                let r_union = expr.1.union(&r_start).union(&r);
                Ok((Node(Stmt::Print(expr), r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::Semicolon), Item::Lexeme(l), r).while_parsing(ELEMENT).max(max_err)),
        }
    }

    fn read_return_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "return statement";

        let r_start = match self.peek() {
            Token(Lexeme::Return, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Return), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::Semicolon, r) => {
                self.advance();
                let r_union = expr.1.union(&r_start).union(&r);
                Ok((Node(Stmt::Return(expr), r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::Semicolon), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_if_else_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "if-else statement";

        let r_start = match self.peek() {
            Token(Lexeme::If, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::If), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        let (true_block, max_err) = match self.read_block() {
            Ok((block, err)) => {
                (block, err.max(max_err))
            }
            Err(err) => return Err(err.max(max_err).while_parsing(ELEMENT)),
        };

        let r_else = match self.peek() {
            Token(Lexeme::Else, r) => { self.advance(); r },
            Token(l, r) => {
                let r_union = expr.1.union(&r_start);
                return Ok((Node(Stmt::If(expr, true_block), r_union), expected(Item::Lexeme(Lexeme::Else), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)))
            },
        };

        match self.read_block() {
            Ok((block, err)) => {
                let r_union = expr.1.union(&r_start).union(&r_else).union(&block.1);
                Ok((Node(Stmt::IfElse(expr, true_block, block), r_union), err.max(max_err).while_parsing(ELEMENT)))
            }
            Err(err) => Err(err.max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_while_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "while statement";

        let r_start = match self.peek() {
            Token(Lexeme::While, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::While), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.read_block() {
            Ok((block, err)) => {
                let r_union = expr.1.union(&r_start).union(&block.1);
                Ok((Node(Stmt::While(expr, block), r_union), err.max(max_err).while_parsing(ELEMENT)))
            }
            Err(err) => Err(err.max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_for_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "for statement";

        let r_start = match self.peek() {
            Token(Lexeme::For, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::For), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (ident, r_ident) = match self.peek() {
            Token(Lexeme::Ident(s), r) => { self.advance(); (s.clone(), r) },
            Token(l, r) => return Err(expected(Item::Ident, Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let r_middle = match self.peek() {
            Token(Lexeme::In, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::In), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.read_block() {
            Ok((block, err)) => {
                let r_union = expr.1.union(&r_start).union(&r_ident).union(&r_middle).union(&block.1);
                Ok((Node(Stmt::For(Node(ident, r_ident), expr, block), r_union), err.max(max_err).while_parsing(ELEMENT)))
            }
            Err(err) => Err(err.max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_decl_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        const ELEMENT: &'static str = "variable declaration";

        let r_start = match self.peek() {
            Token(Lexeme::Var, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Var), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (ident, r_ident) = match self.peek() {
            Token(Lexeme::Ident(s), r) => { self.advance(); (s.clone(), r) },
            Token(l, r) => return Err(expected(Item::Ident, Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let r_assign = match self.peek() {
            Token(Lexeme::Assign, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::Assign), Item::Lexeme(l), r).while_parsing(ELEMENT)),
        };

        let (expr, max_err) = self.read_expr().map_err(|err| err.while_parsing(ELEMENT))?;

        match self.peek() {
            Token(Lexeme::Semicolon, r) => {
                self.advance();
                let r_union = expr.1.union(&r_start).union(&r_ident).union(&r_assign).union(&r);
                Ok((Node(Stmt::Decl(Node(ident, r_ident), expr), r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::Semicolon), Item::Lexeme(l), r).max(max_err).while_parsing(ELEMENT)),
        }
    }

    fn read_stmt(&mut self) -> ParseResult<(Node<Stmt>, ParseError)> {
        let mut this = self.clone();
        let max_err = match this.read_expr_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err))
            },
            Err(err) => err,
        };

        let mut this = self.clone();
        let max_err = match this.read_print_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let mut this = self.clone();
        let max_err = match this.read_if_else_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let mut this = self.clone();
        let max_err = match this.read_while_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let mut this = self.clone();
        let max_err = match this.read_for_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let mut this = self.clone();
        let max_err = match this.read_decl_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let mut this = self.clone();
        let max_err = match this.read_return_stmt() {
            Ok((stmt, err)) => {
                *self = this;
                return Ok((stmt, err.max(max_err)))
            },
            Err(err) => err.max(max_err),
        };

        let next = self.peek();
        Err(expected(Item::Stmt, Item::Lexeme(next.0), next.1).max(max_err))
    }

    fn read_stmts(&mut self) -> ParseResult<(Vec<Node<Stmt>>, ParseError)> {
        let mut stmts = vec![];

        let mut max_err = ParseError::phoney();

        loop {
            let mut this = self.clone();

            match this.read_stmt() { // TODO: Not this
                Ok((stmt, err)) => {
                    *self = this;
                    stmts.push(stmt);
                    max_err = err.max(max_err);
                },
                Err(err) => return Ok((stmts, err.max(max_err))),
            }
        }
    }

    fn read_block(&mut self) -> ParseResult<(Node<Block>, ParseError)> {
        let r_start = match self.peek() {
            Token(Lexeme::LBrace, r) => { self.advance(); r },
            Token(l, r) => return Err(expected(Item::Lexeme(Lexeme::LBrace), Item::Lexeme(l), r)),
        };

        let (stmts, max_err) = self.read_stmts()?;

        match self.peek() {
            Token(Lexeme::RBrace, r) => {
                self.advance();
                let r_union = stmts
                        .iter()
                        .fold(SrcRef::empty(), |r, p| p.1.union(&r))
                        .union(&r_start)
                        .union(&r);
                Ok((Node(Block(stmts), r_union), max_err))
            },
            Token(l, r) => Err(expected(Item::Lexeme(Lexeme::RBrace), Item::Lexeme(l), r).max(max_err)),
        }
    }

    fn read_args(&mut self) -> ParseResult<(Node<Args>, ParseError)> {
        let mut args = vec![];
        let mut r_total = SrcRef::empty();
        let mut max_err = ParseError::Phoney;

        loop {
            match self.peek() {
                Token(Lexeme::Ident(s), r) => {
                    self.advance();
                    r_total = r_total.union(&r);
                    args.push(Node(s.clone(), r));
                },
                Token(l, r) => {
                    max_err = expected(Item::Ident, Item::Lexeme(l), r).max(max_err);
                    break;
                },
            }

            match self.peek() {
                Token(Lexeme::Comma, r) => {
                    self.advance();
                    r_total = r_total.union(&r);
                },
                Token(l, r) => {
                    max_err = expected(Item::Lexeme(Lexeme::Comma), Item::Lexeme(l), r).max(max_err);
                    break;
                },
            }
        }

        Ok((Node(Args(args), r_total), max_err))
    }

    pub fn read_expr_full(&mut self) -> ParseResult<Expr> {
        let (expr, max_err) = match self.read_expr() {
            Ok((expr, max_err)) => (expr, max_err),
            Err(err) => return match self.peek() {
                Token(Lexeme::Eof, _) => Ok(Expr::None),
                _ => Err(err),
            },
        };
        match self.peek() {
            Token(Lexeme::Eof, _) => Ok(expr.0),
            Token(l, r) => Err(expected(Item::End, Item::Lexeme(l), r).max(max_err)),
        }
    }

    pub fn read_stmts_full(&mut self) -> ParseResult<Vec<Node<Stmt>>> {
        let (stmts, max_err) = match self.read_stmts() {
            Ok((stmts, max_err)) => (stmts, max_err),
            Err(err) => return match self.peek() {
                Token(Lexeme::Eof, _) => Ok(vec![]),
                _ => Err(err),
            },
        };
        match self.peek() {
            Token(Lexeme::Eof, _) => Ok(stmts),
            Token(l, r) => Err(expected(Item::End, Item::Lexeme(l), r).max(max_err)),
        }
    }
}
