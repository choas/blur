use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use thiserror::Error;

// Global blur factor for weighted averaging
// blur = 1.0 means pure average (maximum blur)
// blur = 0.9 means recent values count more (default)
// blur = 0.0 means only most recent value counts (no blur)
thread_local! {
    static DECAY: RefCell<f64> = RefCell::new(0.9);
}

pub fn set_decay(decay: f64) {
    DECAY.with(|d| *d.borrow_mut() = decay.clamp(0.0, 1.0));
}

pub fn get_decay() -> f64 {
    DECAY.with(|d| *d.borrow())
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Undefined variable: {0}")]
    UndefinedVar(String),
    #[error("Undefined function: {0}")]
    UndefinedFunc(String),
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Array index out of bounds: {index} for array of size {size}")]
    IndexOutOfBounds { index: i64, size: usize },
    #[error("Invalid operation: {0}")]
    InvalidOp(String),
}

/// A Blur value - stores the history of all assigned values
#[derive(Debug, Clone)]
pub struct BlurValue {
    pub var_type: Type,
    pub history: Vec<f64>, // All values stored as f64 for averaging
    pub bool_history: Vec<bool>, // Separate history for booleans
    pub string_history: Vec<Vec<char>>, // Per-position character history for strings
    pub sharp: bool, // If true, don't average - just use last value (sharp mode)
}

impl BlurValue {
    pub fn new(var_type: Type) -> Self {
        BlurValue {
            var_type,
            history: Vec::new(),
            bool_history: Vec::new(),
            string_history: Vec::new(),
            sharp: false,
        }
    }

    pub fn new_sharp(var_type: Type) -> Self {
        BlurValue {
            var_type,
            history: Vec::new(),
            bool_history: Vec::new(),
            string_history: Vec::new(),
            sharp: true,
        }
    }

    pub fn new_with_value(var_type: Type, value: f64) -> Self {
        BlurValue {
            var_type,
            history: vec![value],
            bool_history: Vec::new(),
            string_history: Vec::new(),
            sharp: false,
        }
    }

    pub fn new_bool(value: bool) -> Self {
        BlurValue {
            var_type: Type::Bool,
            history: Vec::new(),
            bool_history: vec![value],
            string_history: Vec::new(),
            sharp: false,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.sharp {
            // Sharp mode: replace instead of append
            self.history.clear();
        }
        self.history.push(value);
    }

    pub fn push_bool(&mut self, value: bool) {
        if self.sharp {
            self.bool_history.clear();
        }
        self.bool_history.push(value);
    }

    /// Push a string value - adds each non-space character to its position's history
    pub fn push_string(&mut self, s: &str) {
        if self.sharp {
            self.string_history.clear();
        }
        for (i, c) in s.chars().enumerate() {
            // Space is a no-op - doesn't add to history
            if c == ' ' {
                continue;
            }
            // Extend history if needed
            while self.string_history.len() <= i {
                self.string_history.push(Vec::new());
            }
            self.string_history[i].push(c);
        }
    }

    /// Push a string value multiple times (for "str" * n)
    pub fn push_string_times(&mut self, s: &str, times: usize) {
        for _ in 0..times {
            self.push_string(s);
        }
    }

    /// Compute weighted average with decay factor
    /// decay = 1.0: pure average (all weights equal)
    /// decay < 1.0: recent values weighted more (weight = decay^age)
    fn weighted_avg(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let decay = get_decay();
        if decay >= 1.0 {
            // Pure average (fast path)
            return values.iter().sum::<f64>() / values.len() as f64;
        }
        // Weighted average: most recent (last) has weight 1, older has decay^age
        let n = values.len();
        let mut weighted_sum = 0.0;
        let mut weight_total = 0.0;
        for (i, &val) in values.iter().enumerate() {
            let age = (n - 1 - i) as f64; // 0 for most recent
            let weight = decay.powf(age);
            weighted_sum += val * weight;
            weight_total += weight;
        }
        weighted_sum / weight_total
    }

    /// Get the current averaged value
    pub fn get(&self) -> Value {
        match &self.var_type {
            Type::Int => {
                if self.history.is_empty() {
                    Value::Int(0)
                } else {
                    let avg = Self::weighted_avg(&self.history);
                    Value::Int(avg.ceil() as i64)
                }
            }
            Type::Float => {
                if self.history.is_empty() {
                    Value::Float(0.0)
                } else {
                    Value::Float(Self::weighted_avg(&self.history))
                }
            }
            Type::Bool => {
                if self.bool_history.is_empty() {
                    Value::Bool(false)
                } else {
                    let decay = get_decay();
                    if decay >= 1.0 {
                        // Pure average
                        let true_count = self.bool_history.iter().filter(|&&b| b).count();
                        let ratio = true_count as f64 / self.bool_history.len() as f64;
                        Value::Bool(ratio >= 0.5)
                    } else {
                        // Weighted average for bools
                        let n = self.bool_history.len();
                        let mut weighted_true = 0.0;
                        let mut weight_total = 0.0;
                        for (i, &b) in self.bool_history.iter().enumerate() {
                            let age = (n - 1 - i) as f64;
                            let weight = decay.powf(age);
                            if b {
                                weighted_true += weight;
                            }
                            weight_total += weight;
                        }
                        Value::Bool(weighted_true / weight_total >= 0.5)
                    }
                }
            }
            Type::Char => {
                if self.history.is_empty() {
                    Value::Char('\0')
                } else {
                    let avg = Self::weighted_avg(&self.history);
                    Value::Char(avg.ceil() as u8 as char)
                }
            }
            Type::String => {
                if self.string_history.is_empty() {
                    Value::String(String::new())
                } else {
                    let decay = get_decay();
                    let s: String = self.string_history.iter().map(|pos_history| {
                        if pos_history.is_empty() {
                            ' ' // No chars at this position yet
                        } else if decay >= 1.0 {
                            // Pure average
                            let avg = pos_history.iter()
                                .map(|c| *c as u32 as f64)
                                .sum::<f64>() / pos_history.len() as f64;
                            let code = avg.ceil() as u32;
                            char::from_u32(code).unwrap_or(' ')
                        } else {
                            // Weighted average
                            let values: Vec<f64> = pos_history.iter()
                                .map(|c| *c as u32 as f64)
                                .collect();
                            let avg = Self::weighted_avg(&values);
                            let code = avg.ceil() as u32;
                            char::from_u32(code).unwrap_or(' ')
                        }
                    }).collect();
                    Value::String(s)
                }
            }
            Type::Void => Value::Void,
            Type::Array(_, _) => Value::Void, // Arrays are handled separately
        }
    }

    /// Get the raw averaged float value (for increment operations)
    pub fn get_raw(&self) -> f64 {
        Self::weighted_avg(&self.history)
    }
}

/// Runtime value enum for expression evaluation
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Void,
}

impl Value {
    pub fn to_f64(&self) -> f64 {
        match self {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::Char(c) => *c as u8 as f64,
            _ => 0.0,
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Char(c) => *c != '\0',
            _ => false,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Char(_) => "char",
            Value::String(_) => "string",
            Value::Void => "void",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "{}", c),
            Value::String(s) => write!(f, "{}", s),
            Value::Void => write!(f, "void"),
        }
    }
}

/// A scope containing variables
#[derive(Debug, Clone)]
pub struct Scope {
    pub vars: HashMap<String, BlurValue>,
    pub arrays: HashMap<String, Vec<BlurValue>>,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            vars: HashMap::new(),
            arrays: HashMap::new(),
        }
    }
}

/// Control flow signal
pub enum ControlFlow {
    None,
    Return(Value),
}

/// The Blur interpreter
pub struct Interpreter {
    pub functions: HashMap<String, Function>,
    pub scopes: Vec<Scope>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            functions: HashMap::new(),
            scopes: vec![Scope::new()],
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // Register all functions
        for func in &program.functions {
            self.functions.insert(func.name.clone(), func.clone());
        }

        // Call blur() if it exists (the Blur entry point)
        if self.functions.contains_key("blur") {
            self.call_function("blur", vec![])
        } else {
            Ok(Value::Void)
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn current_scope(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    fn get_var(&self, name: &str) -> Result<&BlurValue, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.vars.get(name) {
                return Ok(var);
            }
        }
        Err(RuntimeError::UndefinedVar(name.to_string()))
    }

    fn get_var_mut(&mut self, name: &str) -> Result<&mut BlurValue, RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.vars.contains_key(name) {
                return Ok(scope.vars.get_mut(name).unwrap());
            }
        }
        Err(RuntimeError::UndefinedVar(name.to_string()))
    }

    fn get_array(&self, name: &str) -> Result<&Vec<BlurValue>, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(arr) = scope.arrays.get(name) {
                return Ok(arr);
            }
        }
        Err(RuntimeError::UndefinedVar(name.to_string()))
    }

    fn get_array_mut(&mut self, name: &str) -> Result<&mut Vec<BlurValue>, RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.arrays.contains_key(name) {
                return Ok(scope.arrays.get_mut(name).unwrap());
            }
        }
        Err(RuntimeError::UndefinedVar(name.to_string()))
    }

    fn call_function(&mut self, name: &str, args: Vec<BlurValue>) -> Result<Value, RuntimeError> {
        let func = self.functions.get(name)
            .ok_or_else(|| RuntimeError::UndefinedFunc(name.to_string()))?
            .clone();

        self.push_scope();

        // Bind parameters - history travels with arguments!
        for (i, (_param_type, param_name)) in func.params.iter().enumerate() {
            if i < args.len() {
                // Clone the full BlurValue including history
                self.current_scope().vars.insert(param_name.clone(), args[i].clone());
            }
        }

        // Execute body
        let mut result = Value::Void;
        for stmt in &func.body {
            match self.exec_stmt(stmt)? {
                ControlFlow::Return(v) => {
                    result = v;
                    break;
                }
                ControlFlow::None => {}
            }
        }

        self.pop_scope();
        Ok(result)
    }

    /// Evaluate an expression and return a BlurValue with history (if it's a variable)
    fn eval_expr_as_blur(&mut self, expr: &Expr) -> Result<BlurValue, RuntimeError> {
        match expr {
            // If it's a simple variable reference, clone its full history
            Expr::Var(name) => {
                let var = self.get_var(name)?;
                Ok(var.clone())
            }
            // For array access, clone the element's history
            Expr::ArrayAccess(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                Ok(arr[index as usize].clone())
            }
            // For any other expression, evaluate it and create a new BlurValue
            _ => {
                let value = self.eval_expr(expr)?;
                let mut blur_val = BlurValue::new(Type::Float); // Default to float for expressions
                blur_val.push(value.to_f64());
                Ok(blur_val)
            }
        }
    }

    pub fn exec_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::VarDecl(var_type, name, init) => {
                let mut blur_val = BlurValue::new(var_type.clone());
                if let Some(expr) = init {
                    // Handle StringRepeat specially
                    if let Expr::StringRepeat(str_expr, count_expr) = expr {
                        if let Expr::StringLit(s) = str_expr.as_ref() {
                            let count = self.eval_expr(count_expr)?.to_f64() as usize;
                            blur_val.push_string_times(s, count);
                        }
                    } else {
                        let value = self.eval_expr(expr)?;
                        match var_type {
                            Type::Bool => blur_val.push_bool(value.to_bool()),
                            Type::String => {
                                if let Value::String(s) = value {
                                    blur_val.push_string(&s);
                                }
                            }
                            _ => blur_val.push(value.to_f64()),
                        }
                    }
                }
                self.current_scope().vars.insert(name.clone(), blur_val);
                Ok(ControlFlow::None)
            }

            Stmt::ArrayDecl(elem_type, name, size, init) => {
                let mut arr: Vec<BlurValue> = (0..*size)
                    .map(|_| BlurValue::new(elem_type.clone()))
                    .collect();

                if let Some(values) = init {
                    for (i, expr) in values.iter().enumerate() {
                        if i < arr.len() {
                            let value = self.eval_expr(expr)?;
                            match elem_type {
                                Type::Bool => arr[i].push_bool(value.to_bool()),
                                _ => arr[i].push(value.to_f64()),
                            }
                        }
                    }
                }
                self.current_scope().arrays.insert(name.clone(), arr);
                Ok(ControlFlow::None)
            }

            Stmt::Assign(name, expr) => {
                // Handle StringRepeat specially
                if let Expr::StringRepeat(str_expr, count_expr) = expr {
                    if let Expr::StringLit(s) = str_expr.as_ref() {
                        let count = self.eval_expr(count_expr)?.to_f64() as usize;
                        let var = self.get_var_mut(name)?;
                        var.push_string_times(s, count);
                        return Ok(ControlFlow::None);
                    }
                }
                let value = self.eval_expr(expr)?;
                let var = self.get_var_mut(name)?;
                match &var.var_type {
                    Type::Bool => var.push_bool(value.to_bool()),
                    Type::String => {
                        if let Value::String(s) = value {
                            var.push_string(&s);
                        }
                    }
                    _ => var.push(value.to_f64()),
                }
                Ok(ControlFlow::None)
            }

            Stmt::ArrayAssign(name, index_expr, value_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let value = self.eval_expr(value_expr)?;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                match &elem.var_type {
                    Type::Bool => elem.push_bool(value.to_bool()),
                    _ => elem.push(value.to_f64()),
                }
                Ok(ControlFlow::None)
            }

            Stmt::CompoundAssign(name, op, expr) => {
                let rhs = self.eval_expr(expr)?;
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                let new_val = match op {
                    CompoundOp::AddAssign => current + rhs.to_f64(),
                    CompoundOp::SubAssign => current - rhs.to_f64(),
                    CompoundOp::MulAssign => current * rhs.to_f64(),
                    CompoundOp::DivAssign => {
                        if rhs.to_f64() == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        current / rhs.to_f64()
                    }
                    CompoundOp::ModAssign => current % rhs.to_f64(),
                };
                var.push(new_val);
                Ok(ControlFlow::None)
            }

            Stmt::ArrayCompoundAssign(name, index_expr, op, value_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let rhs = self.eval_expr(value_expr)?;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                let new_val = match op {
                    CompoundOp::AddAssign => current + rhs.to_f64(),
                    CompoundOp::SubAssign => current - rhs.to_f64(),
                    CompoundOp::MulAssign => current * rhs.to_f64(),
                    CompoundOp::DivAssign => {
                        if rhs.to_f64() == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        current / rhs.to_f64()
                    }
                    CompoundOp::ModAssign => current % rhs.to_f64(),
                };
                elem.push(new_val);
                Ok(ControlFlow::None)
            }

            Stmt::PreIncrement(name) | Stmt::PostIncrement(name) => {
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                var.push(current + 1.0);
                Ok(ControlFlow::None)
            }

            Stmt::PreDecrement(name) | Stmt::PostDecrement(name) => {
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                var.push(current - 1.0);
                Ok(ControlFlow::None)
            }

            Stmt::ArrayPreIncrement(name, index_expr) | Stmt::ArrayPostIncrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                elem.push(current + 1.0);
                Ok(ControlFlow::None)
            }

            Stmt::ArrayPreDecrement(name, index_expr) | Stmt::ArrayPostDecrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                elem.push(current - 1.0);
                Ok(ControlFlow::None)
            }

            Stmt::If(cond, then_branch, else_branch) => {
                let cond_val = self.eval_expr(cond)?;
                if cond_val.to_bool() {
                    self.exec_stmt(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.exec_stmt(else_branch)
                } else {
                    Ok(ControlFlow::None)
                }
            }

            Stmt::While(cond, body) => {
                loop {
                    let cond_val = self.eval_expr(cond)?;
                    if !cond_val.to_bool() {
                        break;
                    }
                    match self.exec_stmt(body)? {
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                        ControlFlow::None => {}
                    }
                }
                Ok(ControlFlow::None)
            }

            Stmt::For(init, cond, update, body) => {
                self.push_scope();

                if let Some(init_stmt) = init {
                    self.exec_stmt(init_stmt)?;
                }

                // Safety limit: regular for loops cap at 1000 iterations
                // (blur semantics can cause loops to run ~forever)
                let mut iterations = 0;
                const MAX_ITERATIONS: usize = 1000;

                loop {
                    if iterations >= MAX_ITERATIONS {
                        eprintln!("Warning: for loop hit {} iteration limit (use 'sharp for' for unlimited)", MAX_ITERATIONS);
                        break;
                    }
                    iterations += 1;

                    if let Some(cond_expr) = cond {
                        let cond_val = self.eval_expr(cond_expr)?;
                        if !cond_val.to_bool() {
                            break;
                        }
                    }

                    match self.exec_stmt(body)? {
                        ControlFlow::Return(v) => {
                            self.pop_scope();
                            return Ok(ControlFlow::Return(v));
                        }
                        ControlFlow::None => {}
                    }

                    if let Some(update_stmt) = update {
                        self.exec_stmt(update_stmt)?;
                    }
                }

                self.pop_scope();
                Ok(ControlFlow::None)
            }

            // SharpFor - variables declared in init are NOT averaged (escape hatch)
            Stmt::SharpFor(init, cond, update, body) => {
                self.push_scope();

                // Execute init but mark declared variables as sharp (not averaged)
                if let Some(init_stmt) = init {
                    self.exec_sharp_stmt(init_stmt)?;
                }

                loop {
                    if let Some(cond_expr) = cond {
                        let cond_val = self.eval_expr(cond_expr)?;
                        if !cond_val.to_bool() {
                            break;
                        }
                    }

                    match self.exec_stmt(body)? {
                        ControlFlow::Return(v) => {
                            self.pop_scope();
                            return Ok(ControlFlow::Return(v));
                        }
                        ControlFlow::None => {}
                    }

                    if let Some(update_stmt) = update {
                        self.exec_stmt(update_stmt)?;
                    }
                }

                self.pop_scope();
                Ok(ControlFlow::None)
            }

            Stmt::Block(stmts) => {
                self.push_scope();
                for stmt in stmts {
                    match self.exec_stmt(stmt)? {
                        ControlFlow::Return(v) => {
                            self.pop_scope();
                            return Ok(ControlFlow::Return(v));
                        }
                        ControlFlow::None => {}
                    }
                }
                self.pop_scope();
                Ok(ControlFlow::None)
            }

            Stmt::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(ControlFlow::None)
            }

            Stmt::Print(exprs) => {
                let values: Vec<String> = exprs
                    .iter()
                    .map(|e| self.eval_expr(e).map(|v| v.to_string()))
                    .collect::<Result<Vec<_>, _>>()?;
                println!("{}", values.join(" "));
                Ok(ControlFlow::None)
            }

            Stmt::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.eval_expr(e)?
                } else {
                    Value::Void
                };
                Ok(ControlFlow::Return(value))
            }
        }
    }

    /// Execute a statement in "sharp" mode - variable declarations are not averaged
    fn exec_sharp_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::VarDecl(var_type, name, init) => {
                let mut blur_val = BlurValue::new_sharp(var_type.clone());
                if let Some(expr) = init {
                    let value = self.eval_expr(expr)?;
                    match var_type {
                        Type::Bool => blur_val.push_bool(value.to_bool()),
                        Type::String => {
                            if let Value::String(s) = value {
                                blur_val.push_string(&s);
                            }
                        }
                        _ => blur_val.push(value.to_f64()),
                    }
                }
                self.current_scope().vars.insert(name.clone(), blur_val);
                Ok(ControlFlow::None)
            }
            // For non-declaration statements, just execute normally
            _ => self.exec_stmt(stmt),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::IntLit(n) => Ok(Value::Int(*n)),
            Expr::FloatLit(f) => Ok(Value::Float(*f)),
            Expr::BoolLit(b) => Ok(Value::Bool(*b)),
            Expr::CharLit(c) => Ok(Value::Char(*c)),
            Expr::StringLit(s) => Ok(Value::String(s.clone())),

            Expr::StringRepeat(str_expr, count_expr) => {
                let s = self.eval_expr(str_expr)?;
                let count = self.eval_expr(count_expr)?.to_f64() as usize;
                if let Value::String(str_val) = s {
                    Ok(Value::String(str_val.repeat(count)))
                } else {
                    Ok(Value::String(String::new()))
                }
            }

            Expr::Var(name) => {
                let var = self.get_var(name)?;
                Ok(var.get())
            }

            Expr::ArrayAccess(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                Ok(arr[index as usize].get())
            }

            Expr::BinOp(left, op, right) => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.eval_binop(l, *op, r)
            }

            Expr::UnaryOp(op, expr) => {
                let v = self.eval_expr(expr)?;
                match op {
                    UnaryOp::Neg => Ok(Value::Float(-v.to_f64())),
                    UnaryOp::Not => Ok(Value::Bool(!v.to_bool())),
                }
            }

            Expr::PreIncrement(name) => {
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                var.push(current + 1.0);
                Ok(var.get())
            }

            Expr::PreDecrement(name) => {
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                var.push(current - 1.0);
                Ok(var.get())
            }

            Expr::PostIncrement(name) => {
                let var = self.get_var_mut(name)?;
                let old_val = var.get();
                let current = var.get_raw();
                var.push(current + 1.0);
                Ok(old_val)
            }

            Expr::PostDecrement(name) => {
                let var = self.get_var_mut(name)?;
                let old_val = var.get();
                let current = var.get_raw();
                var.push(current - 1.0);
                Ok(old_val)
            }

            Expr::ArrayPreIncrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                elem.push(current + 1.0);
                Ok(elem.get())
            }

            Expr::ArrayPreDecrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                elem.push(current - 1.0);
                Ok(elem.get())
            }

            Expr::ArrayPostIncrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let old_val = elem.get();
                let current = elem.get_raw();
                elem.push(current + 1.0);
                Ok(old_val)
            }

            Expr::ArrayPostDecrement(name, index_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let old_val = elem.get();
                let current = elem.get_raw();
                elem.push(current - 1.0);
                Ok(old_val)
            }

            Expr::Call(name, args) => {
                // Built-in get_blur() function - returns current blur factor
                if name == "get_blur" {
                    return Ok(Value::Float(get_decay()));
                }

                // Built-in blurstr() function - blurs multiple strings together
                if name == "blurstr" {
                    let mut blur_val = BlurValue::new(Type::String);
                    for arg in args {
                        // Handle StringRepeat specially
                        if let Expr::StringRepeat(str_expr, count_expr) = arg {
                            if let Expr::StringLit(s) = str_expr.as_ref() {
                                let count = self.eval_expr(count_expr)?.to_f64() as usize;
                                blur_val.push_string_times(s, count);
                                continue;
                            }
                        }
                        let value = self.eval_expr(arg)?;
                        if let Value::String(s) = value {
                            blur_val.push_string(&s);
                        }
                    }
                    return Ok(blur_val.get());
                }

                // Collect BlurValues with full history
                let arg_values: Vec<BlurValue> = args
                    .iter()
                    .map(|a| self.eval_expr_as_blur(a))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(name, arg_values)
            }

            Expr::Assign(name, value_expr) => {
                let value = self.eval_expr(value_expr)?;
                let var = self.get_var_mut(name)?;
                match &var.var_type {
                    Type::Bool => var.push_bool(value.to_bool()),
                    _ => var.push(value.to_f64()),
                }
                Ok(var.get())
            }

            Expr::ArrayAssign(name, index_expr, value_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let value = self.eval_expr(value_expr)?;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                match &elem.var_type {
                    Type::Bool => elem.push_bool(value.to_bool()),
                    _ => elem.push(value.to_f64()),
                }
                Ok(elem.get())
            }

            Expr::CompoundAssign(name, op, value_expr) => {
                let rhs = self.eval_expr(value_expr)?;
                let var = self.get_var_mut(name)?;
                let current = var.get_raw();
                let new_val = match op {
                    CompoundOp::AddAssign => current + rhs.to_f64(),
                    CompoundOp::SubAssign => current - rhs.to_f64(),
                    CompoundOp::MulAssign => current * rhs.to_f64(),
                    CompoundOp::DivAssign => {
                        if rhs.to_f64() == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        current / rhs.to_f64()
                    }
                    CompoundOp::ModAssign => current % rhs.to_f64(),
                };
                var.push(new_val);
                Ok(var.get())
            }

            Expr::ArrayCompoundAssign(name, index_expr, op, value_expr) => {
                let index = self.eval_expr(index_expr)?.to_f64() as i64;
                let rhs = self.eval_expr(value_expr)?;
                let arr = self.get_array_mut(name)?;
                if index < 0 || index as usize >= arr.len() {
                    return Err(RuntimeError::IndexOutOfBounds { index, size: arr.len() });
                }
                let elem = &mut arr[index as usize];
                let current = elem.get_raw();
                let new_val = match op {
                    CompoundOp::AddAssign => current + rhs.to_f64(),
                    CompoundOp::SubAssign => current - rhs.to_f64(),
                    CompoundOp::MulAssign => current * rhs.to_f64(),
                    CompoundOp::DivAssign => {
                        if rhs.to_f64() == 0.0 {
                            return Err(RuntimeError::DivisionByZero);
                        }
                        current / rhs.to_f64()
                    }
                    CompoundOp::ModAssign => current % rhs.to_f64(),
                };
                elem.push(new_val);
                Ok(elem.get())
            }
        }
    }

    fn eval_binop(&self, left: Value, op: BinOp, right: Value) -> Result<Value, RuntimeError> {
        let l = left.to_f64();
        let r = right.to_f64();

        match op {
            BinOp::Add => Ok(Value::Float(l + r)),
            BinOp::Sub => Ok(Value::Float(l - r)),
            BinOp::Mul => Ok(Value::Float(l * r)),
            BinOp::Div => {
                if r == 0.0 {
                    return Err(RuntimeError::DivisionByZero);
                }
                Ok(Value::Float(l / r))
            }
            BinOp::Mod => Ok(Value::Float(l % r)),
            BinOp::Eq => Ok(Value::Bool((l - r).abs() < f64::EPSILON)),
            BinOp::Ne => Ok(Value::Bool((l - r).abs() >= f64::EPSILON)),
            BinOp::Lt => Ok(Value::Bool(l < r)),
            BinOp::Gt => Ok(Value::Bool(l > r)),
            BinOp::Le => Ok(Value::Bool(l <= r)),
            BinOp::Ge => Ok(Value::Bool(l >= r)),
            BinOp::And => Ok(Value::Bool(left.to_bool() && right.to_bool())),
            BinOp::Or => Ok(Value::Bool(left.to_bool() || right.to_bool())),
        }
    }
}
