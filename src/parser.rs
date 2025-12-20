use crate::ast::*;
use crate::lexer::Token;
use logos::Logos;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("Expected {expected}, found {found}")]
    Expected { expected: String, found: String },
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Invalid expression")]
    InvalidExpr,
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let tokens: Vec<Token> = Token::lexer(source)
            .filter_map(|t| t.ok())
            .collect();
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expect(&mut self, expected: Token) -> Result<Token, ParseError> {
        match self.peek() {
            Some(t) if std::mem::discriminant(t) == std::mem::discriminant(&expected) => {
                Ok(self.advance().unwrap())
            }
            Some(t) => Err(ParseError::Expected {
                expected: expected.to_string(),
                found: t.to_string(),
            }),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn check(&self, expected: &Token) -> bool {
        matches!(self.peek(), Some(t) if std::mem::discriminant(t) == std::mem::discriminant(expected))
    }

    fn is_type(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Int | Token::Float | Token::Bool | Token::Char | Token::StringType | Token::Void)
        )
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.advance() {
            Some(Token::Int) => Ok(Type::Int),
            Some(Token::Float) => Ok(Type::Float),
            Some(Token::Bool) => Ok(Type::Bool),
            Some(Token::Char) => Ok(Type::Char),
            Some(Token::StringType) => Ok(Type::String),
            Some(Token::Void) => Ok(Type::Void),
            Some(t) => Err(ParseError::UnexpectedToken(t.to_string())),
            None => Err(ParseError::UnexpectedEof),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        while self.peek().is_some() {
            functions.push(self.parse_function()?);
        }
        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let return_type = self.parse_type()?;
        let name = match self.advance() {
            Some(Token::Identifier(s)) => s,
            Some(t) => return Err(ParseError::Expected {
                expected: "identifier".to_string(),
                found: t.to_string(),
            }),
            None => return Err(ParseError::UnexpectedEof),
        };

        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                let param_type = self.parse_type()?;
                let param_name = match self.advance() {
                    Some(Token::Identifier(s)) => s,
                    Some(t) => return Err(ParseError::Expected {
                        expected: "identifier".to_string(),
                        found: t.to_string(),
                    }),
                    None => return Err(ParseError::UnexpectedEof),
                };
                params.push((param_type, param_name));
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(Token::RParen)?;

        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;

        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        // Variable declaration
        if self.is_type() {
            let var_type = self.parse_type()?;
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(t) => return Err(ParseError::Expected {
                    expected: "identifier".to_string(),
                    found: t.to_string(),
                }),
                None => return Err(ParseError::UnexpectedEof),
            };

            // Check for array declaration
            if self.check(&Token::LBracket) {
                self.advance();
                let size = match self.advance() {
                    Some(Token::IntLiteral(n)) => n as usize,
                    Some(t) => return Err(ParseError::Expected {
                        expected: "array size".to_string(),
                        found: t.to_string(),
                    }),
                    None => return Err(ParseError::UnexpectedEof),
                };
                self.expect(Token::RBracket)?;

                let init = if self.check(&Token::Assign) {
                    self.advance();
                    self.expect(Token::LBrace)?;
                    let mut values = Vec::new();
                    if !self.check(&Token::RBrace) {
                        loop {
                            values.push(self.parse_expr()?);
                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    self.expect(Token::RBrace)?;
                    Some(values)
                } else {
                    None
                };
                self.expect(Token::Semicolon)?;
                return Ok(Stmt::ArrayDecl(var_type, name, size, init));
            }

            let init = if self.check(&Token::Assign) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(Token::Semicolon)?;
            return Ok(Stmt::VarDecl(var_type, name, init));
        }

        // If statement
        if self.check(&Token::If) {
            self.advance();
            self.expect(Token::LParen)?;
            let cond = self.parse_expr()?;
            self.expect(Token::RParen)?;
            let then_branch = Box::new(self.parse_stmt()?);
            let else_branch = if self.check(&Token::Else) {
                self.advance();
                Some(Box::new(self.parse_stmt()?))
            } else {
                None
            };
            return Ok(Stmt::If(cond, then_branch, else_branch));
        }

        // While loop
        if self.check(&Token::While) {
            self.advance();
            self.expect(Token::LParen)?;
            let cond = self.parse_expr()?;
            self.expect(Token::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::While(cond, body));
        }

        // Sharp for loop (escape hatch - no averaging on loop variable)
        if self.check(&Token::Sharp) {
            self.advance();
            self.expect(Token::For)?;
            self.expect(Token::LParen)?;

            // Init
            let init = if self.check(&Token::Semicolon) {
                self.advance();
                None
            } else {
                let stmt = self.parse_for_init()?;
                Some(Box::new(stmt))
            };

            // Condition
            let cond = if self.check(&Token::Semicolon) {
                self.advance();
                None
            } else {
                let expr = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                Some(expr)
            };

            // Update
            let update = if self.check(&Token::RParen) {
                None
            } else {
                let stmt = self.parse_for_update()?;
                Some(Box::new(stmt))
            };

            self.expect(Token::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::SharpFor(init, cond, update, body));
        }

        // For loop
        if self.check(&Token::For) {
            self.advance();
            self.expect(Token::LParen)?;

            // Init
            let init = if self.check(&Token::Semicolon) {
                self.advance();
                None
            } else {
                let stmt = self.parse_for_init()?;
                Some(Box::new(stmt))
            };

            // Condition
            let cond = if self.check(&Token::Semicolon) {
                self.advance();
                None
            } else {
                let expr = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                Some(expr)
            };

            // Update
            let update = if self.check(&Token::RParen) {
                None
            } else {
                let stmt = self.parse_for_update()?;
                Some(Box::new(stmt))
            };

            self.expect(Token::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::For(init, cond, update, body));
        }

        // Block
        if self.check(&Token::LBrace) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.check(&Token::RBrace) {
                stmts.push(self.parse_stmt()?);
            }
            self.expect(Token::RBrace)?;
            return Ok(Stmt::Block(stmts));
        }

        // Print
        if self.check(&Token::Print) {
            self.advance();
            self.expect(Token::LParen)?;
            let mut args = Vec::new();
            if !self.check(&Token::RParen) {
                loop {
                    args.push(self.parse_expr()?);
                    if !self.check(&Token::Comma) {
                        break;
                    }
                    self.advance();
                }
            }
            self.expect(Token::RParen)?;
            self.expect(Token::Semicolon)?;
            return Ok(Stmt::Print(args));
        }

        // Return
        if self.check(&Token::Return) {
            self.advance();
            let value = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect(Token::Semicolon)?;
            return Ok(Stmt::Return(value));
        }

        // Expression statement (assignment, increment, function call, etc.)
        let stmt = self.parse_expr_stmt()?;
        self.expect(Token::Semicolon)?;
        Ok(stmt)
    }

    fn parse_for_init(&mut self) -> Result<Stmt, ParseError> {
        if self.is_type() {
            let var_type = self.parse_type()?;
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(t) => return Err(ParseError::Expected {
                    expected: "identifier".to_string(),
                    found: t.to_string(),
                }),
                None => return Err(ParseError::UnexpectedEof),
            };
            let init = if self.check(&Token::Assign) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(Token::Semicolon)?;
            Ok(Stmt::VarDecl(var_type, name, init))
        } else {
            let stmt = self.parse_expr_stmt()?;
            self.expect(Token::Semicolon)?;
            Ok(stmt)
        }
    }

    fn parse_for_update(&mut self) -> Result<Stmt, ParseError> {
        self.parse_expr_stmt()
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, ParseError> {
        // Check for pre-increment/decrement
        if self.check(&Token::PlusPlus) {
            self.advance();
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(t) => return Err(ParseError::Expected {
                    expected: "identifier".to_string(),
                    found: t.to_string(),
                }),
                None => return Err(ParseError::UnexpectedEof),
            };
            if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                return Ok(Stmt::ArrayPreIncrement(name, index));
            }
            return Ok(Stmt::PreIncrement(name));
        }

        if self.check(&Token::MinusMinus) {
            self.advance();
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                Some(t) => return Err(ParseError::Expected {
                    expected: "identifier".to_string(),
                    found: t.to_string(),
                }),
                None => return Err(ParseError::UnexpectedEof),
            };
            if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                return Ok(Stmt::ArrayPreDecrement(name, index));
            }
            return Ok(Stmt::PreDecrement(name));
        }

        // Must be identifier-based statement or function call
        if let Some(Token::Identifier(name)) = self.peek().cloned() {
            self.advance();

            // Array access
            if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;

                // Array post-increment/decrement
                if self.check(&Token::PlusPlus) {
                    self.advance();
                    return Ok(Stmt::ArrayPostIncrement(name, index));
                }
                if self.check(&Token::MinusMinus) {
                    self.advance();
                    return Ok(Stmt::ArrayPostDecrement(name, index));
                }

                // Array assignment
                if self.check(&Token::Assign) {
                    self.advance();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::ArrayAssign(name, index, value));
                }

                // Array compound assignment
                if let Some(op) = self.try_parse_compound_op() {
                    let value = self.parse_expr()?;
                    return Ok(Stmt::ArrayCompoundAssign(name, index, op, value));
                }
            }

            // Post-increment/decrement
            if self.check(&Token::PlusPlus) {
                self.advance();
                return Ok(Stmt::PostIncrement(name));
            }
            if self.check(&Token::MinusMinus) {
                self.advance();
                return Ok(Stmt::PostDecrement(name));
            }

            // Simple assignment
            if self.check(&Token::Assign) {
                self.advance();
                let value = self.parse_expr()?;
                return Ok(Stmt::Assign(name, value));
            }

            // Compound assignment
            if let Some(op) = self.try_parse_compound_op() {
                let value = self.parse_expr()?;
                return Ok(Stmt::CompoundAssign(name, op, value));
            }

            // Function call as statement
            if self.check(&Token::LParen) {
                self.advance();
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.check(&Token::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(Token::RParen)?;
                return Ok(Stmt::Expr(Expr::Call(name, args)));
            }

            // Just a variable expression (shouldn't happen often)
            return Ok(Stmt::Expr(Expr::Var(name)));
        }

        // Generic expression
        let expr = self.parse_expr()?;
        Ok(Stmt::Expr(expr))
    }

    fn try_parse_compound_op(&mut self) -> Option<CompoundOp> {
        let op = match self.peek() {
            Some(Token::PlusAssign) => Some(CompoundOp::AddAssign),
            Some(Token::MinusAssign) => Some(CompoundOp::SubAssign),
            Some(Token::StarAssign) => Some(CompoundOp::MulAssign),
            Some(Token::SlashAssign) => Some(CompoundOp::DivAssign),
            Some(Token::PercentAssign) => Some(CompoundOp::ModAssign),
            _ => None,
        };
        if op.is_some() {
            self.advance();
        }
        op
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.check(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.check(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::Equal) => BinOp::Eq,
                Some(Token::NotEqual) => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Some(Token::Less) => BinOp::Lt,
                Some(Token::Greater) => BinOp::Gt,
                Some(Token::LessEqual) => BinOp::Le,
                Some(Token::GreaterEqual) => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    // String repetition: "str" * n
                    if matches!(left, Expr::StringLit(_)) {
                        left = Expr::StringRepeat(Box::new(left), Box::new(right));
                    } else {
                        left = Expr::BinOp(Box::new(left), BinOp::Mul, Box::new(right));
                    }
                }
                Some(Token::Slash) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::BinOp(Box::new(left), BinOp::Div, Box::new(right));
                }
                Some(Token::Percent) => {
                    self.advance();
                    let right = self.parse_unary()?;
                    left = Expr::BinOp(Box::new(left), BinOp::Mod, Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&Token::Minus) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr)));
        }
        if self.check(&Token::Not) {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr)));
        }
        if self.check(&Token::PlusPlus) {
            self.advance();
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                _ => return Err(ParseError::InvalidExpr),
            };
            if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                return Ok(Expr::ArrayPreIncrement(name, Box::new(index)));
            }
            return Ok(Expr::PreIncrement(name));
        }
        if self.check(&Token::MinusMinus) {
            self.advance();
            let name = match self.advance() {
                Some(Token::Identifier(s)) => s,
                _ => return Err(ParseError::InvalidExpr),
            };
            if self.check(&Token::LBracket) {
                self.advance();
                let index = self.parse_expr()?;
                self.expect(Token::RBracket)?;
                return Ok(Expr::ArrayPreDecrement(name, Box::new(index)));
            }
            return Ok(Expr::PreDecrement(name));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&Token::LBracket) {
                if let Expr::Var(name) = expr {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;

                    // Check for post-increment/decrement
                    if self.check(&Token::PlusPlus) {
                        self.advance();
                        return Ok(Expr::ArrayPostIncrement(name, Box::new(index)));
                    }
                    if self.check(&Token::MinusMinus) {
                        self.advance();
                        return Ok(Expr::ArrayPostDecrement(name, Box::new(index)));
                    }
                    expr = Expr::ArrayAccess(name, Box::new(index));
                } else {
                    break;
                }
            } else if self.check(&Token::PlusPlus) {
                if let Expr::Var(name) = expr {
                    self.advance();
                    return Ok(Expr::PostIncrement(name));
                }
                break;
            } else if self.check(&Token::MinusMinus) {
                if let Expr::Var(name) = expr {
                    self.advance();
                    return Ok(Expr::PostDecrement(name));
                }
                break;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(Token::IntLiteral(n)) => {
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Some(Token::FloatLiteral(n)) => {
                self.advance();
                Ok(Expr::FloatLit(n))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Expr::BoolLit(true))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Expr::BoolLit(false))
            }
            Some(Token::CharLiteral(c)) => {
                self.advance();
                Ok(Expr::CharLit(c))
            }
            Some(Token::StringLiteral(s)) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Some(Token::Identifier(name)) => {
                self.advance();
                // Function call
                if self.check(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if !self.check(&Token::Comma) {
                                break;
                            }
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(t) => Err(ParseError::UnexpectedToken(t.to_string())),
            None => Err(ParseError::UnexpectedEof),
        }
    }
}
