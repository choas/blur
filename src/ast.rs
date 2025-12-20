#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Char,
    String,
    Void,
    Array(Box<Type>, usize), // Array of type with size
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    CharLit(char),
    StringLit(String),

    // Variable access
    Var(String),
    ArrayAccess(String, Box<Expr>), // array[index]

    // Binary operations
    BinOp(Box<Expr>, BinOp, Box<Expr>),

    // Unary operations
    UnaryOp(UnaryOp, Box<Expr>),

    // Pre/post increment/decrement
    PreIncrement(String),
    PreDecrement(String),
    PostIncrement(String),
    PostDecrement(String),

    // Array pre/post increment/decrement
    ArrayPreIncrement(String, Box<Expr>),
    ArrayPreDecrement(String, Box<Expr>),
    ArrayPostIncrement(String, Box<Expr>),
    ArrayPostDecrement(String, Box<Expr>),

    // Function call
    Call(String, Vec<Expr>),

    // String repetition: "str" * n (adds to history n times)
    StringRepeat(Box<Expr>, Box<Expr>),

    // Assignment expressions (for use in for-loop updates)
    Assign(String, Box<Expr>),
    ArrayAssign(String, Box<Expr>, Box<Expr>), // array[index] = value
    CompoundAssign(String, CompoundOp, Box<Expr>),
    ArrayCompoundAssign(String, Box<Expr>, CompoundOp, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompoundOp {
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    // Variable declaration with optional initializer
    VarDecl(Type, String, Option<Expr>),

    // Array declaration: type name[size] or type name[size] = {values}
    ArrayDecl(Type, String, usize, Option<Vec<Expr>>),

    // Assignment
    Assign(String, Expr),
    ArrayAssign(String, Expr, Expr), // array[index] = value

    // Compound assignment (+=, -=, etc.)
    CompoundAssign(String, CompoundOp, Expr),
    ArrayCompoundAssign(String, Expr, CompoundOp, Expr),

    // Increment/Decrement statements
    PreIncrement(String),
    PreDecrement(String),
    PostIncrement(String),
    PostDecrement(String),
    ArrayPreIncrement(String, Expr),
    ArrayPreDecrement(String, Expr),
    ArrayPostIncrement(String, Expr),
    ArrayPostDecrement(String, Expr),

    // Control flow
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Box<Stmt>>, Box<Stmt>),
    // Sharp for - variables in the loop header are NOT averaged (escape hatch)
    SharpFor(Option<Box<Stmt>>, Option<Expr>, Option<Box<Stmt>>, Box<Stmt>),

    // Block
    Block(Vec<Stmt>),

    // Expression statement
    Expr(Expr),

    // Print (built-in)
    Print(Vec<Expr>),

    // Return
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}
