// SPDX-License-Identifier: AGPL-3.0-or-later
//! Recalculation: turning stored formulas back into values.
//!
//! [`crate::sheet`] keeps a formula *and* the result the source cached for it,
//! which is enough to display a workbook faithfully and not enough to reason
//! about one. An agent that changes a cell needs to know what that changes, and
//! an agent handed a spreadsheet needs to know whether the numbers in it still
//! follow from the formulas beside them. Both are this module.
//!
//! # What it computes, and what it refuses to
//!
//! The evaluator is deliberately small and deliberately honest about its edges:
//!
//! - **Dependencies decide the order**, not the order cells appear in. A cell
//!   is evaluated by evaluating what it refers to first, depth first, so a
//!   total above its inputs is as correct as one below them.
//! - **A cycle is reported, not resolved.** Excel answers a circular reference
//!   with a warning and a zero; guessing here would replace a value the source
//!   computed with one this build invented. The cell keeps its cached value and
//!   the cycle is named in the report.
//! - **An unknown function is `#NAME?`**, which is exactly what a spreadsheet
//!   says when it meets one, rather than an error that stops the recalculation.
//!   The rest of the sheet still computes.
//!
//! Everything it cannot do is a *reported* result rather than a silent wrong
//! number, which is the property that matters when an agent is going to act on
//! the answer.

use std::collections::HashMap;

use crate::sheet::{Cell, CellError, Sheet, Value, Workbook, parse_a1};

/// How a formula failed to parse. Distinct from [`CellError`]: this is the
/// expression being malformed, not the computation producing an error value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcError {
    /// The expression ended before it was finished.
    UnexpectedEnd,
    /// A character that cannot begin a term.
    Unexpected(String),
    /// A `(` with no `)`.
    UnclosedParen,
    /// Nesting past the depth limit, which a hostile file can otherwise use to
    /// overflow the stack.
    TooDeep,
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalcError::UnexpectedEnd => write!(f, "formula ended unexpectedly"),
            CalcError::Unexpected(what) => write!(f, "unexpected '{what}' in formula"),
            CalcError::UnclosedParen => write!(f, "unclosed parenthesis"),
            CalcError::TooDeep => write!(f, "formula nests too deeply"),
        }
    }
}

impl std::error::Error for CalcError {}

/// How deep an expression may nest. Well past anything a person writes, and
/// far short of the stack.
const MAX_DEPTH: usize = 64;

/// A parsed formula.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Text(String),
    Bool(bool),
    /// A literal error, as written in `=NA()` style formulas or read back from
    /// a file that stored one.
    Error(CellError),
    /// A single cell, optionally on another sheet.
    Ref {
        sheet: Option<String>,
        row: u32,
        column: u32,
    },
    /// A rectangular region, which is only meaningful as a function argument.
    Range {
        sheet: Option<String>,
        first_row: u32,
        first_column: u32,
        last_row: u32,
        last_column: u32,
    },
    /// A name defined at workbook or sheet scope.
    Name(String),
    Unary {
        op: char,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    /// `&`, string concatenation.
    Concat,
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl BinaryOp {
    /// Binding power. Higher binds tighter.
    fn precedence(self) -> u8 {
        match self {
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessOrEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterOrEqual => 1,
            BinaryOp::Concat => 2,
            BinaryOp::Add | BinaryOp::Subtract => 3,
            BinaryOp::Multiply | BinaryOp::Divide => 4,
            BinaryOp::Power => 5,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a formula. The leading `=` is optional, because a file stores the
/// expression without one and a person types it with one.
pub fn parse(source: &str) -> Result<Expr, CalcError> {
    let source = source.trim();
    let source = source.strip_prefix('=').unwrap_or(source);
    let mut parser = Parser {
        bytes: source.as_bytes(),
        at: 0,
        depth: 0,
    };
    let expr = parser.expression(0)?;
    parser.skip_space();
    match parser.at < parser.bytes.len() {
        true => Err(CalcError::Unexpected(parser.rest_snippet())),
        false => Ok(expr),
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn skip_space(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn rest_snippet(&self) -> String {
        String::from_utf8_lossy(&self.bytes[self.at..])
            .chars()
            .take(12)
            .collect()
    }

    /// Pratt loop: a term, then operators of at least `min_power`.
    fn expression(&mut self, min_power: u8) -> Result<Expr, CalcError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(CalcError::TooDeep);
        }

        let mut left = self.term()?;
        loop {
            self.skip_space();
            let Some((op, width)) = self.binary_op() else {
                break;
            };
            if op.precedence() < min_power {
                break;
            }
            self.at += width;

            // `^` is right-associative, so it recurses at its own power rather
            // than one above it: 2^3^2 is 2^(3^2).
            let next_power = match op {
                BinaryOp::Power => op.precedence(),
                _ => op.precedence() + 1,
            };
            let right = self.expression(next_power)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        self.depth -= 1;
        Ok(left)
    }

    fn binary_op(&self) -> Option<(BinaryOp, usize)> {
        let rest = &self.bytes[self.at.min(self.bytes.len())..];
        match rest {
            [b'<', b'=', ..] => Some((BinaryOp::LessOrEqual, 2)),
            [b'>', b'=', ..] => Some((BinaryOp::GreaterOrEqual, 2)),
            [b'<', b'>', ..] => Some((BinaryOp::NotEqual, 2)),
            [b'+', ..] => Some((BinaryOp::Add, 1)),
            [b'-', ..] => Some((BinaryOp::Subtract, 1)),
            [b'*', ..] => Some((BinaryOp::Multiply, 1)),
            [b'/', ..] => Some((BinaryOp::Divide, 1)),
            [b'^', ..] => Some((BinaryOp::Power, 1)),
            [b'&', ..] => Some((BinaryOp::Concat, 1)),
            [b'=', ..] => Some((BinaryOp::Equal, 1)),
            [b'<', ..] => Some((BinaryOp::Less, 1)),
            [b'>', ..] => Some((BinaryOp::Greater, 1)),
            _ => None,
        }
    }

    fn term(&mut self) -> Result<Expr, CalcError> {
        self.skip_space();
        let Some(byte) = self.peek() else {
            return Err(CalcError::UnexpectedEnd);
        };

        // Unary sign, and a postfix `%` handled by the caller of `term` below.
        if byte == b'-' || byte == b'+' {
            self.at += 1;
            let operand = self.term()?;
            let expr = match byte {
                b'-' => Expr::Unary {
                    op: '-',
                    operand: Box::new(operand),
                },
                _ => operand,
            };
            return Ok(self.percent(expr));
        }

        if byte == b'(' {
            self.at += 1;
            let inner = self.expression(0)?;
            self.skip_space();
            if self.peek() != Some(b')') {
                return Err(CalcError::UnclosedParen);
            }
            self.at += 1;
            return Ok(self.percent(inner));
        }

        if byte == b'"' {
            return Ok(Expr::Text(self.string()?));
        }

        if byte == b'#' {
            return self.error_literal();
        }

        if byte.is_ascii_digit() || byte == b'.' {
            let number = self.number()?;
            return Ok(self.percent(number));
        }

        if byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$' || byte == b'\'' {
            return self.name_or_reference();
        }

        Err(CalcError::Unexpected(self.rest_snippet()))
    }

    /// `50%` is `0.5`. A postfix operator, so it binds tighter than anything.
    fn percent(&mut self, expr: Expr) -> Expr {
        self.skip_space();
        match self.peek() {
            Some(b'%') => {
                self.at += 1;
                Expr::Binary {
                    op: BinaryOp::Divide,
                    left: Box::new(expr),
                    right: Box::new(Expr::Number(100.0)),
                }
            }
            _ => expr,
        }
    }

    fn string(&mut self) -> Result<String, CalcError> {
        self.at += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(CalcError::UnexpectedEnd),
                // A doubled quote is one literal quote, as everywhere else in
                // spreadsheets.
                Some(b'"') if self.bytes.get(self.at + 1) == Some(&b'"') => {
                    out.push('"');
                    self.at += 2;
                }
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(_) => {
                    let start = self.at;
                    while self.at < self.bytes.len() && self.bytes[self.at] != b'"' {
                        self.at += 1;
                    }
                    out.push_str(&String::from_utf8_lossy(&self.bytes[start..self.at]));
                }
            }
        }
    }

    fn number(&mut self) -> Result<Expr, CalcError> {
        let start = self.at;
        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit() || byte == b'.') {
            self.at += 1;
        }
        // An exponent, but only when it is one: `1e5` is a number and `A1e` is
        // not, so the `e` must be followed by digits or a sign and digits.
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mark = self.at;
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            match matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
                true => {
                    while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
                        self.at += 1;
                    }
                }
                false => self.at = mark,
            }
        }

        String::from_utf8_lossy(&self.bytes[start..self.at])
            .parse::<f64>()
            .map(Expr::Number)
            .map_err(|_| {
                CalcError::Unexpected(String::from_utf8_lossy(&self.bytes[start..self.at]).into())
            })
    }

    /// `#DIV/0!` and friends, written literally in a formula.
    fn error_literal(&mut self) -> Result<Expr, CalcError> {
        let start = self.at;
        self.at += 1;
        while matches!(self.peek(), Some(byte) if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'!' | b'?' | b'_'))
        {
            self.at += 1;
        }
        let code = String::from_utf8_lossy(&self.bytes[start..self.at]).to_string();
        match CellError::from_code(&code) {
            Some(error) => Ok(Expr::Error(error)),
            None => Err(CalcError::Unexpected(code)),
        }
    }

    /// A function call, a cell reference, a range, or a defined name.
    fn name_or_reference(&mut self) -> Result<Expr, CalcError> {
        let mut sheet = None;

        // A sheet qualifier, quoted when the name has spaces in it.
        let mark = self.at;
        if self.peek() == Some(b'\'') {
            let quoted = self.string_single()?;
            if self.peek() == Some(b'!') {
                self.at += 1;
                sheet = Some(quoted);
            } else {
                self.at = mark;
                return Err(CalcError::Unexpected(self.rest_snippet()));
            }
        }

        let word = self.word();
        if word.is_empty() && sheet.is_none() {
            return Err(CalcError::Unexpected(self.rest_snippet()));
        }

        if sheet.is_none() && self.peek() == Some(b'!') {
            self.at += 1;
            sheet = Some(word);
            let target = self.word();
            return self.finish_reference(sheet, target);
        }

        // A `(` makes it a call. Sheet-qualified calls do not exist, so this is
        // only reachable with no qualifier.
        self.skip_space();
        if sheet.is_none() && self.peek() == Some(b'(') {
            self.at += 1;
            let args = self.arguments()?;
            return Ok(Expr::Call {
                name: word.to_ascii_uppercase(),
                args,
            });
        }

        self.finish_reference(sheet, word)
    }

    fn string_single(&mut self) -> Result<String, CalcError> {
        self.at += 1;
        let start = self.at;
        while self.at < self.bytes.len() && self.bytes[self.at] != b'\'' {
            self.at += 1;
        }
        if self.at >= self.bytes.len() {
            return Err(CalcError::UnexpectedEnd);
        }
        let value = String::from_utf8_lossy(&self.bytes[start..self.at]).to_string();
        self.at += 1;
        Ok(value)
    }

    /// A bare word: letters, digits, `_`, `.` and the `$` of an absolute
    /// reference.
    fn word(&mut self) -> String {
        let start = self.at;
        while matches!(self.peek(), Some(byte) if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$'))
        {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.at]).to_string()
    }

    /// Decide whether a word is a cell, the start of a range, a boolean, or a
    /// defined name.
    fn finish_reference(&mut self, sheet: Option<String>, word: String) -> Result<Expr, CalcError> {
        if self.peek() == Some(b':') {
            let mark = self.at;
            self.at += 1;
            let second = self.word();
            if let (Some(from), Some(to)) = (parse_a1(&word), parse_a1(&second)) {
                return Ok(Expr::Range {
                    sheet,
                    first_row: from.0.min(to.0),
                    first_column: from.1.min(to.1),
                    last_row: from.0.max(to.0),
                    last_column: from.1.max(to.1),
                });
            }
            self.at = mark;
        }

        if let Some((row, column)) = parse_a1(&word) {
            return Ok(Expr::Ref { sheet, row, column });
        }

        match word.to_ascii_uppercase().as_str() {
            "TRUE" => Ok(Expr::Bool(true)),
            "FALSE" => Ok(Expr::Bool(false)),
            _ => Ok(Expr::Name(word)),
        }
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, CalcError> {
        let mut args = Vec::new();
        self.skip_space();
        if self.peek() == Some(b')') {
            self.at += 1;
            return Ok(args);
        }
        loop {
            args.push(self.expression(0)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b')') => {
                    self.at += 1;
                    return Ok(args);
                }
                None => return Err(CalcError::UnclosedParen),
                Some(_) => return Err(CalcError::Unexpected(self.rest_snippet())),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// A cell's address across the workbook.
type Address = (usize, u32, u32);

/// What a recalculation found.
#[derive(Debug, Clone, Default)]
pub struct RecalcReport {
    /// Cells that were evaluated.
    pub evaluated: usize,
    /// Cells whose recomputed value differs from the one the file cached,
    /// as `(sheet name, A1 reference, cached, computed)`.
    pub disagreements: Vec<Disagreement>,
    /// Formulas that would not parse, as `(sheet name, reference, why)`.
    pub unparsed: Vec<(String, String, CalcError)>,
    /// Cells that refer to themselves, directly or through others. Their cached
    /// values are left alone.
    pub cycles: Vec<(String, String)>,
}

/// One cell where the stored result and the recomputed one disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct Disagreement {
    pub sheet: String,
    pub reference: String,
    pub cached: Value,
    pub computed: Value,
}

/// Recompute every formula in the workbook, reporting what it found.
///
/// The workbook is not modified. A recalculation that quietly overwrote the
/// values a source cached would destroy the evidence needed to notice that they
/// had drifted, which is the main thing anyone would run this for.
pub fn check(workbook: &Workbook) -> RecalcReport {
    let mut engine = Engine::new(workbook);
    let mut report = RecalcReport::default();

    for (index, sheet) in workbook.sheets.iter().enumerate() {
        for cell in sheet.formulas() {
            let address = (index, cell.row, cell.column);
            let computed = engine.value_of(address);
            report.evaluated += 1;

            match engine.outcome(address) {
                Outcome::Cycle => report.cycles.push((sheet.name.clone(), cell.reference())),
                Outcome::Unparsed(why) => {
                    report
                        .unparsed
                        .push((sheet.name.clone(), cell.reference(), why))
                }
                Outcome::Value => {
                    if !same_value(&computed, &cell.value) {
                        report.disagreements.push(Disagreement {
                            sheet: sheet.name.clone(),
                            reference: cell.reference(),
                            cached: cell.value.clone(),
                            computed,
                        });
                    }
                }
            }
        }
    }

    report
}

/// Recompute every formula and write the results into the workbook.
///
/// Cells in a cycle and formulas that will not parse keep the value they had:
/// there is no answer to write, and a zero would be an invention.
pub fn recalculate(workbook: &mut Workbook) -> RecalcReport {
    let report = check(workbook);
    let updates: HashMap<(String, String), Value> = report
        .disagreements
        .iter()
        .map(|d| ((d.sheet.clone(), d.reference.clone()), d.computed.clone()))
        .collect();

    for sheet in &mut workbook.sheets {
        for cell in &mut sheet.cells {
            if cell.formula.is_none() {
                continue;
            }
            if let Some(value) = updates.get(&(sheet.name.clone(), cell.reference())) {
                cell.value = value.clone();
            }
        }
    }
    report
}

/// Two values are the same when they are the same kind and the same content.
///
/// Numbers compare with a relative tolerance: a cached result was written by
/// another program's arithmetic, and demanding bit equality would report every
/// floating-point sum in every workbook as a disagreement.
fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            let scale = a.abs().max(b.abs()).max(1.0);
            (a - b).abs() <= scale * 1e-9
        }
        _ => left == right,
    }
}

enum Outcome {
    Value,
    Cycle,
    Unparsed(CalcError),
}

enum State {
    InProgress,
    Done(Value),
    Cycle,
    Unparsed(CalcError),
}

struct Engine<'a> {
    workbook: &'a Workbook,
    /// Sheet name to index, for resolving qualified references.
    by_name: HashMap<String, usize>,
    state: HashMap<Address, State>,
}

impl<'a> Engine<'a> {
    fn new(workbook: &'a Workbook) -> Engine<'a> {
        Engine {
            workbook,
            by_name: workbook
                .sheets
                .iter()
                .enumerate()
                .map(|(index, sheet)| (sheet.name.to_ascii_lowercase(), index))
                .collect(),
            state: HashMap::new(),
        }
    }

    fn sheet(&self, index: usize) -> Option<&'a Sheet> {
        self.workbook.sheets.get(index)
    }

    fn cell(&self, address: Address) -> Option<&'a Cell> {
        self.sheet(address.0)?.cell(address.1, address.2)
    }

    fn outcome(&self, address: Address) -> Outcome {
        match self.state.get(&address) {
            Some(State::Cycle) => Outcome::Cycle,
            Some(State::Unparsed(why)) => Outcome::Unparsed(why.clone()),
            _ => Outcome::Value,
        }
    }

    /// The value of a cell, computing it if it holds a formula.
    fn value_of(&mut self, address: Address) -> Value {
        match self.state.get(&address) {
            Some(State::Done(value)) => return value.clone(),
            // Reached while it is still being computed: the cell depends on
            // itself. Report it and hand back what the file cached rather than
            // inventing a value for it.
            Some(State::InProgress) => {
                self.state.insert(address, State::Cycle);
                return self.cached(address);
            }
            Some(State::Cycle) | Some(State::Unparsed(_)) => return self.cached(address),
            None => {}
        }

        let Some(cell) = self.cell(address) else {
            return Value::Empty;
        };
        let Some(formula) = cell.formula.clone() else {
            let value = cell.value.clone();
            self.state.insert(address, State::Done(value.clone()));
            return value;
        };

        self.state.insert(address, State::InProgress);
        let value = match parse(&formula) {
            Ok(expr) => self.eval(&expr, address.0),
            Err(why) => {
                self.state.insert(address, State::Unparsed(why));
                return self.cached(address);
            }
        };

        // A cycle discovered while evaluating this cell wins over the value.
        if matches!(self.state.get(&address), Some(State::Cycle)) {
            return self.cached(address);
        }
        self.state.insert(address, State::Done(value.clone()));
        value
    }

    fn cached(&self, address: Address) -> Value {
        self.cell(address)
            .map(|cell| cell.value.clone())
            .unwrap_or(Value::Empty)
    }

    fn resolve_sheet(&self, name: &Option<String>, current: usize) -> Option<usize> {
        match name {
            None => Some(current),
            Some(name) => self.by_name.get(&name.to_ascii_lowercase()).copied(),
        }
    }

    fn eval(&mut self, expr: &Expr, sheet: usize) -> Value {
        match expr {
            Expr::Number(number) => Value::Number(*number),
            Expr::Text(text) => Value::Text(text.clone()),
            Expr::Bool(value) => Value::Bool(*value),
            Expr::Error(error) => Value::Error(*error),

            Expr::Ref {
                sheet: name,
                row,
                column,
            } => match self.resolve_sheet(name, sheet) {
                Some(index) => self.value_of((index, *row, *column)),
                None => Value::Error(CellError::Ref),
            },

            // A range in scalar position is not a value. Excel resolves it by
            // implicit intersection; refusing is the honest answer for a build
            // that does not implement that.
            Expr::Range { .. } => Value::Error(CellError::Value),

            Expr::Name(name) => self.resolve_name(name, sheet),

            Expr::Unary { op, operand } => {
                let value = self.eval(operand, sheet);
                match (op, to_number(&value)) {
                    (_, Err(error)) => Value::Error(error),
                    ('-', Ok(number)) => Value::Number(-number),
                    (_, Ok(number)) => Value::Number(number),
                }
            }

            Expr::Binary { op, left, right } => {
                let left = self.eval(left, sheet);
                let right = self.eval(right, sheet);
                binary(*op, &left, &right)
            }

            Expr::Call { name, args } => self.call(name, args, sheet),
        }
    }

    /// A defined name expands to the formula it stands for.
    fn resolve_name(&mut self, name: &str, sheet: usize) -> Value {
        let target = self
            .workbook
            .defined_names
            .iter()
            .find(|defined| defined.name.eq_ignore_ascii_case(name));
        let Some(target) = target else {
            return Value::Error(CellError::Name);
        };
        match parse(&target.refers_to) {
            Ok(expr) => self.eval(&expr, sheet),
            Err(_) => Value::Error(CellError::Name),
        }
    }

    /// Flatten a function argument into the values it stands for, so that
    /// `SUM(A1:A9, 3)` and `SUM(A1, A2, 3)` are the same to the function.
    fn spread(&mut self, expr: &Expr, sheet: usize) -> Vec<Value> {
        match expr {
            Expr::Range {
                sheet: name,
                first_row,
                first_column,
                last_row,
                last_column,
            } => {
                let Some(index) = self.resolve_sheet(name, sheet) else {
                    return vec![Value::Error(CellError::Ref)];
                };
                let mut values = Vec::new();
                for row in *first_row..=*last_row {
                    for column in *first_column..=*last_column {
                        // Only cells that exist: a range over a mostly empty
                        // region should cost its contents, not its area.
                        if self.cell((index, row, column)).is_some() {
                            values.push(self.value_of((index, row, column)));
                        }
                    }
                }
                values
            }
            other => vec![self.eval(other, sheet)],
        }
    }

    fn call(&mut self, name: &str, args: &[Expr], sheet: usize) -> Value {
        // The logical functions decide for themselves what to evaluate, so
        // they are handled before the arguments are.
        match name {
            "IF" => {
                if args.len() < 2 {
                    return Value::Error(CellError::Value);
                }
                let condition = self.eval(&args[0], sheet);
                if let Value::Error(error) = condition {
                    return Value::Error(error);
                }
                return match to_bool(&condition) {
                    Err(error) => Value::Error(error),
                    Ok(true) => self.eval(&args[1], sheet),
                    Ok(false) => match args.get(2) {
                        Some(otherwise) => self.eval(otherwise, sheet),
                        None => Value::Bool(false),
                    },
                };
            }
            "IFERROR" => {
                if args.len() != 2 {
                    return Value::Error(CellError::Value);
                }
                let value = self.eval(&args[0], sheet);
                return match value {
                    Value::Error(_) => self.eval(&args[1], sheet),
                    other => other,
                };
            }
            _ => {}
        }

        let mut values = Vec::new();
        for arg in args {
            values.extend(self.spread(arg, sheet));
        }
        apply(name, &values)
    }
}

/// Numeric coercion.
///
/// Empty is zero and a boolean is one or nought, as everywhere in spreadsheets.
/// Text is coerced when it parses as a number — `"1" + 1` is `2` — and is
/// `#VALUE!` when it does not, which is the behaviour that keeps a column of
/// labels out of a sum.
fn to_number(value: &Value) -> Result<f64, CellError> {
    match value {
        Value::Number(number) => Ok(*number),
        Value::Empty => Ok(0.0),
        Value::Bool(true) => Ok(1.0),
        Value::Bool(false) => Ok(0.0),
        Value::Text(text) => text.trim().parse::<f64>().map_err(|_| CellError::Value),
        Value::Error(error) => Err(*error),
    }
}

fn to_bool(value: &Value) -> Result<bool, CellError> {
    match value {
        Value::Bool(value) => Ok(*value),
        Value::Number(number) => Ok(*number != 0.0),
        Value::Empty => Ok(false),
        Value::Text(text) => match text.trim().to_ascii_uppercase().as_str() {
            "TRUE" => Ok(true),
            "FALSE" => Ok(false),
            _ => Err(CellError::Value),
        },
        Value::Error(error) => Err(*error),
    }
}

fn to_text(value: &Value) -> String {
    value.to_text()
}

fn binary(op: BinaryOp, left: &Value, right: &Value) -> Value {
    // An error operand propagates rather than being coerced, so the first
    // failure in a chain is the one reported.
    if let Value::Error(error) = left {
        return Value::Error(*error);
    }
    if let Value::Error(error) = right {
        return Value::Error(*error);
    }

    if op == BinaryOp::Concat {
        return Value::Text(format!("{}{}", to_text(left), to_text(right)));
    }

    if matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessOrEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterOrEqual
    ) {
        return compare(op, left, right);
    }

    let (a, b) = match (to_number(left), to_number(right)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(error), _) | (_, Err(error)) => return Value::Error(error),
    };

    match op {
        BinaryOp::Add => Value::Number(a + b),
        BinaryOp::Subtract => Value::Number(a - b),
        BinaryOp::Multiply => Value::Number(a * b),
        BinaryOp::Divide => match b == 0.0 {
            true => Value::Error(CellError::Div0),
            false => Value::Number(a / b),
        },
        BinaryOp::Power => {
            let result = a.powf(b);
            match result.is_finite() {
                true => Value::Number(result),
                false => Value::Error(CellError::Num),
            }
        }
        _ => Value::Error(CellError::Value),
    }
}

fn compare(op: BinaryOp, left: &Value, right: &Value) -> Value {
    // Numbers compare as numbers and text as text; a number and a piece of text
    // are never equal, which is the same rule that keeps `42` and `"42"` apart
    // in the cell model.
    let ordering = match (left, right) {
        (Value::Text(a), Value::Text(b)) => {
            a.to_ascii_lowercase().partial_cmp(&b.to_ascii_lowercase())
        }
        (Value::Text(_), _) | (_, Value::Text(_)) => {
            return match op {
                BinaryOp::Equal => Value::Bool(false),
                BinaryOp::NotEqual => Value::Bool(true),
                _ => Value::Error(CellError::Value),
            };
        }
        _ => match (to_number(left), to_number(right)) {
            (Ok(a), Ok(b)) => a.partial_cmp(&b),
            (Err(error), _) | (_, Err(error)) => return Value::Error(error),
        },
    };

    let Some(ordering) = ordering else {
        return Value::Error(CellError::Num);
    };
    Value::Bool(match op {
        BinaryOp::Equal => ordering.is_eq(),
        BinaryOp::NotEqual => ordering.is_ne(),
        BinaryOp::Less => ordering.is_lt(),
        BinaryOp::LessOrEqual => ordering.is_le(),
        BinaryOp::Greater => ordering.is_gt(),
        BinaryOp::GreaterOrEqual => ordering.is_ge(),
        _ => false,
    })
}

/// The function library.
///
/// Small on purpose. These are the functions that appear in the spreadsheets a
/// document pipeline actually meets; anything else answers `#NAME?`, which is
/// what a spreadsheet says when it meets a function it does not have, and lets
/// the rest of the sheet compute regardless.
fn apply(name: &str, args: &[Value]) -> Value {
    /// Every argument as a number, propagating the first error.
    fn numbers(args: &[Value]) -> Result<Vec<f64>, CellError> {
        args.iter()
            .filter(|value| !value.is_empty())
            .map(to_number)
            .collect()
    }

    /// Only the arguments that are already numbers, which is what the
    /// aggregates use: text in a range is skipped rather than fatal.
    fn numeric(args: &[Value]) -> Result<Vec<f64>, CellError> {
        let mut out = Vec::new();
        for value in args {
            match value {
                Value::Error(error) => return Err(*error),
                Value::Number(number) => out.push(*number),
                _ => {}
            }
        }
        Ok(out)
    }

    fn one(args: &[Value]) -> Result<&Value, CellError> {
        args.first().ok_or(CellError::Value)
    }

    let result = (|| -> Result<Value, CellError> {
        Ok(match name {
            "SUM" => Value::Number(numeric(args)?.iter().sum()),
            "PRODUCT" => Value::Number(numeric(args)?.iter().product()),
            "AVERAGE" => {
                let values = numeric(args)?;
                match values.is_empty() {
                    true => return Err(CellError::Div0),
                    false => Value::Number(values.iter().sum::<f64>() / values.len() as f64),
                }
            }
            "MIN" => {
                let values = numeric(args)?;
                Value::Number(values.into_iter().fold(f64::INFINITY, f64::min))
                    .finite_or(Value::Number(0.0))
            }
            "MAX" => {
                let values = numeric(args)?;
                Value::Number(values.into_iter().fold(f64::NEG_INFINITY, f64::max))
                    .finite_or(Value::Number(0.0))
            }
            "COUNT" => Value::Number(numeric(args)?.len() as f64),
            "COUNTA" => Value::Number(args.iter().filter(|v| !v.is_empty()).count() as f64),
            "ABS" => Value::Number(to_number(one(args)?)?.abs()),
            "SQRT" => {
                let number = to_number(one(args)?)?;
                match number < 0.0 {
                    true => return Err(CellError::Num),
                    false => Value::Number(number.sqrt()),
                }
            }
            "INT" => Value::Number(to_number(one(args)?)?.floor()),
            "ROUND" => {
                let values = numbers(args)?;
                let (number, digits) = (
                    *values.first().ok_or(CellError::Value)?,
                    values.get(1).copied().unwrap_or(0.0),
                );
                let scale = 10f64.powi(digits as i32);
                Value::Number((number * scale).round() / scale)
            }
            "AND" => {
                let mut all = true;
                for value in args.iter().filter(|v| !v.is_empty()) {
                    all &= to_bool(value)?;
                }
                Value::Bool(all)
            }
            "OR" => {
                let mut any = false;
                for value in args.iter().filter(|v| !v.is_empty()) {
                    any |= to_bool(value)?;
                }
                Value::Bool(any)
            }
            "NOT" => Value::Bool(!to_bool(one(args)?)?),
            "TRUE" => Value::Bool(true),
            "FALSE" => Value::Bool(false),
            "NA" => return Err(CellError::NotAvailable),
            "LEN" => Value::Number(to_text(one(args)?).chars().count() as f64),
            "UPPER" => Value::Text(to_text(one(args)?).to_uppercase()),
            "LOWER" => Value::Text(to_text(one(args)?).to_lowercase()),
            "TRIM" => Value::Text(to_text(one(args)?).trim().to_string()),
            "CONCAT" | "CONCATENATE" => {
                Value::Text(args.iter().map(to_text).collect::<Vec<_>>().join(""))
            }
            "LEFT" | "RIGHT" => {
                let text = to_text(one(args)?);
                let count = match args.get(1) {
                    Some(value) => to_number(value)?.max(0.0) as usize,
                    None => 1,
                };
                let chars: Vec<char> = text.chars().collect();
                let count = count.min(chars.len());
                let taken: String = match name {
                    "LEFT" => chars[..count].iter().collect(),
                    _ => chars[chars.len() - count..].iter().collect(),
                };
                Value::Text(taken)
            }
            "MID" => {
                let text = to_text(one(args)?);
                let start = to_number(args.get(1).ok_or(CellError::Value)?)?;
                let count = to_number(args.get(2).ok_or(CellError::Value)?)?;
                if start < 1.0 || count < 0.0 {
                    return Err(CellError::Value);
                }
                let chars: Vec<char> = text.chars().collect();
                let from = ((start as usize) - 1).min(chars.len());
                let to = from.saturating_add(count as usize).min(chars.len());
                Value::Text(chars[from..to].iter().collect())
            }
            _ => return Err(CellError::Name),
        })
    })();

    match result {
        Ok(value) => value,
        Err(error) => Value::Error(error),
    }
}

impl Value {
    /// A number that came out infinite — `MIN` of nothing, say — replaced by a
    /// stated fallback.
    fn finite_or(self, fallback: Value) -> Value {
        match &self {
            Value::Number(number) if !number.is_finite() => fallback,
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::Cell;

    fn sheet_with(cells: &[(u32, u32, Value, Option<&str>)]) -> Workbook {
        let mut sheet = Sheet::new("Sheet1");
        for (row, column, value, formula) in cells {
            sheet.set(Cell {
                row: *row,
                column: *column,
                value: value.clone(),
                formula: formula.map(str::to_string),
                number_format: None,
                href: None,
            });
        }
        Workbook {
            sheets: vec![sheet],
            defined_names: Vec::new(),
        }
    }

    /// Evaluate a formula in a sheet holding A1=2, A2=3, A3=5, B1="x".
    fn eval(formula: &str) -> Value {
        let mut workbook = sheet_with(&[
            (0, 0, Value::Number(2.0), None),
            (1, 0, Value::Number(3.0), None),
            (2, 0, Value::Number(5.0), None),
            (0, 1, Value::Text("x".into()), None),
            (9, 9, Value::Empty, Some(formula)),
        ]);
        recalculate(&mut workbook);
        workbook.sheets[0].cell(9, 9).unwrap().value.clone()
    }

    #[test]
    fn arithmetic_follows_precedence() {
        assert_eq!(eval("1+2*3"), Value::Number(7.0));
        assert_eq!(eval("(1+2)*3"), Value::Number(9.0));
        assert_eq!(eval("-2^2"), Value::Number(4.0));
        // Right associative, so this is 2^(3^2) and not (2^3)^2.
        assert_eq!(eval("2^3^2"), Value::Number(512.0));
        assert_eq!(eval("10%"), Value::Number(0.1));
        assert_eq!(eval("1e3+1"), Value::Number(1001.0));
    }

    #[test]
    fn references_and_ranges_resolve() {
        assert_eq!(eval("A1+A2"), Value::Number(5.0));
        assert_eq!(eval("SUM(A1:A3)"), Value::Number(10.0));
        assert_eq!(eval("$A$1+A2"), Value::Number(5.0));
        assert_eq!(eval("SUM(A1:A3, 10)"), Value::Number(20.0));
        assert_eq!(eval("AVERAGE(A1:A3)"), Value::Number(10.0 / 3.0));
        // A range where a single value belongs is refused rather than guessed.
        assert_eq!(eval("A1:A3+1"), Value::Error(CellError::Value));
    }

    #[test]
    fn division_by_zero_is_an_error_value_not_an_infinity() {
        assert_eq!(eval("1/0"), Value::Error(CellError::Div0));
        // And it propagates rather than being coerced away.
        assert_eq!(eval("1/0+1"), Value::Error(CellError::Div0));
        assert_eq!(eval("IFERROR(1/0, 42)"), Value::Number(42.0));
    }

    #[test]
    fn text_stays_out_of_arithmetic_unless_it_is_a_number() {
        assert_eq!(eval("B1+1"), Value::Error(CellError::Value));
        assert_eq!(eval("\"12\"+1"), Value::Number(13.0));
        assert_eq!(eval("\"a\"&\"b\""), Value::Text("ab".into()));
        assert_eq!(eval("A1&\"x\""), Value::Text("2x".into()));
    }

    #[test]
    fn comparisons_and_conditionals() {
        assert_eq!(eval("A1<A2"), Value::Bool(true));
        assert_eq!(eval("A1=2"), Value::Bool(true));
        assert_eq!(eval("A1<>2"), Value::Bool(false));
        // A number and a piece of text are never equal.
        assert_eq!(eval("A1=B1"), Value::Bool(false));
        assert_eq!(
            eval("IF(A1>1, \"big\", \"small\")"),
            Value::Text("big".into())
        );
        assert_eq!(
            eval("IF(A1>99, \"big\", \"small\")"),
            Value::Text("small".into())
        );
        assert_eq!(eval("AND(TRUE, A1=2)"), Value::Bool(true));
        assert_eq!(eval("NOT(TRUE)"), Value::Bool(false));
    }

    #[test]
    fn the_text_functions_work_on_characters_not_bytes() {
        assert_eq!(eval("LEN(\"héllo\")"), Value::Number(5.0));
        assert_eq!(eval("LEFT(\"héllo\", 2)"), Value::Text("hé".into()));
        assert_eq!(eval("RIGHT(\"héllo\", 2)"), Value::Text("lo".into()));
        assert_eq!(eval("MID(\"héllo\", 2, 3)"), Value::Text("éll".into()));
        assert_eq!(eval("UPPER(\"ab\")"), Value::Text("AB".into()));
        assert_eq!(eval("CONCAT(\"a\", 1, TRUE)"), Value::Text("a1TRUE".into()));
    }

    #[test]
    fn an_unknown_function_is_a_name_error_not_a_failure() {
        assert_eq!(eval("VLOOKUP(1,A1:A3,1)"), Value::Error(CellError::Name));
        // The rest of the sheet still computes.
        assert_eq!(eval("SUM(A1:A3)"), Value::Number(10.0));
    }

    #[test]
    fn evaluation_order_follows_dependencies_not_position() {
        // The total sits *above* the inputs it depends on.
        let mut workbook = sheet_with(&[
            (0, 0, Value::Empty, Some("A2+A3")),
            (1, 0, Value::Number(3.0), None),
            (2, 0, Value::Empty, Some("A4*2")),
            (3, 0, Value::Number(5.0), None),
        ]);
        let report = recalculate(&mut workbook);

        assert_eq!(
            workbook.sheets[0].cell(0, 0).unwrap().value,
            Value::Number(13.0)
        );
        assert_eq!(
            workbook.sheets[0].cell(2, 0).unwrap().value,
            Value::Number(10.0)
        );
        assert!(report.cycles.is_empty());
    }

    #[test]
    fn a_cycle_is_reported_and_the_cached_value_left_alone() {
        let mut workbook = sheet_with(&[
            (0, 0, Value::Number(99.0), Some("A2+1")),
            (1, 0, Value::Number(98.0), Some("A1+1")),
        ]);
        let report = recalculate(&mut workbook);

        assert!(!report.cycles.is_empty(), "the cycle should be reported");
        // Not overwritten with a guess.
        assert_eq!(
            workbook.sheets[0].cell(0, 0).unwrap().value,
            Value::Number(99.0)
        );
    }

    #[test]
    fn check_finds_a_cached_value_that_no_longer_follows() {
        // The formula says 5; the file claims 500.
        let workbook = sheet_with(&[
            (0, 0, Value::Number(2.0), None),
            (1, 0, Value::Number(3.0), None),
            (2, 0, Value::Number(500.0), Some("A1+A2")),
        ]);
        let report = check(&workbook);

        assert_eq!(report.evaluated, 1);
        assert_eq!(report.disagreements.len(), 1);
        assert_eq!(report.disagreements[0].reference, "A3");
        assert_eq!(report.disagreements[0].cached, Value::Number(500.0));
        assert_eq!(report.disagreements[0].computed, Value::Number(5.0));
        // `check` does not modify the workbook it is auditing.
        assert_eq!(
            workbook.sheets[0].cell(2, 0).unwrap().value,
            Value::Number(500.0)
        );
    }

    #[test]
    fn a_correct_cached_value_is_not_reported_as_drift() {
        let workbook = sheet_with(&[
            (0, 0, Value::Number(0.1), None),
            (1, 0, Value::Number(0.2), None),
            // 0.1 + 0.2 is not 0.3 in binary floating point; a workbook that
            // stores the rounded result is right, not drifted.
            (2, 0, Value::Number(0.3), Some("A1+A2")),
        ]);
        assert!(check(&workbook).disagreements.is_empty());
    }

    #[test]
    fn cross_sheet_references_resolve_by_name() {
        let mut rates = Sheet::new("My Rates");
        rates.set(Cell::new(0, 0, Value::Number(0.2)));
        let mut main = Sheet::new("Main");
        main.set(Cell {
            row: 0,
            column: 0,
            value: Value::Empty,
            formula: Some("'My Rates'!A1*100".into()),
            number_format: None,
            href: None,
        });
        main.set(Cell {
            row: 1,
            column: 0,
            value: Value::Empty,
            formula: Some("Nowhere!A1".into()),
            number_format: None,
            href: None,
        });

        let mut workbook = Workbook {
            sheets: vec![main, rates],
            defined_names: Vec::new(),
        };
        recalculate(&mut workbook);

        assert_eq!(
            workbook.sheets[0].cell(0, 0).unwrap().value,
            Value::Number(20.0)
        );
        // A sheet that is not there is a reference error, not a panic.
        assert_eq!(
            workbook.sheets[0].cell(1, 0).unwrap().value,
            Value::Error(CellError::Ref)
        );
    }

    #[test]
    fn a_defined_name_expands_to_what_it_stands_for() {
        let mut sheet = Sheet::new("Sheet1");
        sheet.set(Cell::new(0, 0, Value::Number(0.25)));
        sheet.set(Cell {
            row: 1,
            column: 0,
            value: Value::Empty,
            formula: Some("Tax_Rate*4".into()),
            number_format: None,
            href: None,
        });
        let mut workbook = Workbook {
            sheets: vec![sheet],
            defined_names: vec![crate::sheet::DefinedName {
                name: "Tax_Rate".into(),
                refers_to: "Sheet1!$A$1".into(),
                sheet: None,
            }],
        };
        recalculate(&mut workbook);

        assert_eq!(
            workbook.sheets[0].cell(1, 0).unwrap().value,
            Value::Number(1.0)
        );
    }

    #[test]
    fn a_formula_that_will_not_parse_is_reported_rather_than_panicking() {
        let workbook = sheet_with(&[(0, 0, Value::Number(1.0), Some("SUM(A1"))]);
        let report = check(&workbook);
        assert_eq!(report.unparsed.len(), 1);
        assert_eq!(report.unparsed[0].2, CalcError::UnclosedParen);
    }

    #[test]
    fn hostile_nesting_is_refused_rather_than_overflowing_the_stack() {
        let deep = format!("{}1{}", "(".repeat(500), ")".repeat(500));
        assert_eq!(parse(&deep), Err(CalcError::TooDeep));

        let deep_calls = format!("{}1{}", "SUM(".repeat(500), ")".repeat(500));
        assert_eq!(parse(&deep_calls), Err(CalcError::TooDeep));
    }

    #[test]
    fn parsing_never_panics_on_rubbish() {
        for source in [
            "",
            "=",
            "((",
            "))",
            "1+",
            "+",
            "\"unterminated",
            "A1:",
            ":A1",
            "#NOPE!",
            "SUM(,)",
            "'unclosed",
            "1..2",
            "&&&",
            "A1!!B2",
            "%",
            "1e",
            "IF(",
        ] {
            let _ = parse(source);
        }
    }
}
