//! AutoLISP values: integers (32-bit, wrapping as AutoLISP's do), reals, strings, symbols,
//! lists built of cons cells, entity names, selection sets, files and functions.
//!
//! Values are shared (`Arc`) so a session stays `Send`. A cons chain is dropped iteratively:
//! a list a million long would otherwise overflow the stack when it goes away.

use std::sync::{Arc, Mutex};

use cadcraft_doc::Handle;

use super::LispError;

/// A symbol: its name in upper case (AutoLISP symbols are case-insensitive).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sym(pub Arc<str>);

impl Sym {
    pub fn new(name: &str) -> Sym {
        Sym(Arc::from(name.to_ascii_uppercase().as_str()))
    }
    pub fn name(&self) -> &str {
        &self.0
    }
}

/// A cons cell.
#[derive(Debug)]
pub struct Cons {
    pub car: Value,
    pub cdr: Value,
}

impl Drop for Cons {
    fn drop(&mut self) {
        // Unlink the rest of the chain one cell at a time while this is its only owner.
        let mut next = std::mem::replace(&mut self.cdr, Value::Nil);
        while let Value::Cons(c) = next {
            match Arc::try_unwrap(c) {
                Ok(mut cell) => next = std::mem::replace(&mut cell.cdr, Value::Nil),
                Err(_) => break,
            }
        }
    }
}

/// A selection set: the entities it holds, in order. `ssadd` and `ssdel` change it in place.
#[derive(Clone, Debug)]
pub struct PickSet {
    pub id: u32,
    pub items: Arc<Mutex<Vec<Handle>>>,
}

/// A function defined in LISP (`defun`, `lambda`).
#[derive(Debug)]
pub struct Func {
    pub name: Option<Sym>,
    pub params: Vec<Sym>,
    /// Local variables (after the `/` in the argument list): bound to nil during a call.
    pub locals: Vec<Sym>,
    /// The body forms, as a list.
    pub body: Value,
}

/// A built-in function. `special` forms get their arguments unevaluated.
pub struct Builtin {
    pub name: &'static str,
    pub kind: BuiltinKind,
}

impl std::fmt::Debug for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Subr: {}>", self.name)
    }
}

#[derive(Clone, Copy)]
pub enum BuiltinKind {
    /// Evaluated arguments to a value.
    Pure(fn(&[Value]) -> Result<Value, LispError>),
    /// Evaluated arguments, with the interpreter (and its session) at hand.
    Ctx(fn(&mut super::eval::Ctx<'_>, Vec<Value>) -> Result<super::eval::Flow, LispError>),
    /// A special form, handled by the evaluator itself.
    Special(super::eval::Special),
}

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Nil,
    Int(i32),
    Real(f64),
    Str(Arc<str>),
    Sym(Sym),
    Cons(Arc<Cons>),
    Ename(Handle),
    PickSet(PickSet),
    File(u32),
    Subr(&'static Builtin),
    Func(Arc<Func>),
}

impl Value {
    pub fn t() -> Value {
        Value::Sym(Sym::new("T"))
    }
    pub fn bool(b: bool) -> Value {
        if b { Value::t() } else { Value::Nil }
    }
    pub fn str(s: &str) -> Value {
        Value::Str(Arc::from(s))
    }
    pub fn sym(s: &str) -> Value {
        Value::Sym(Sym::new(s))
    }
    pub fn cons(car: Value, cdr: Value) -> Value {
        Value::Cons(Arc::new(Cons { car, cdr }))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    /// A proper list of `items`.
    pub fn list(items: impl IntoIterator<Item = Value, IntoIter: DoubleEndedIterator>) -> Value {
        items.into_iter().rev().fold(Value::Nil, |acc, v| Value::cons(v, acc))
    }
    /// `items` followed by `tail` (a dotted list when `tail` is an atom).
    pub fn list_with_tail(items: Vec<Value>, tail: Value) -> Value {
        items.into_iter().rev().fold(tail, |acc, v| Value::cons(v, acc))
    }
    /// A point as a list of reals.
    pub fn point(p: &[f64]) -> Value {
        Value::list(p.iter().map(|v| Value::Real(*v)).collect::<Vec<_>>())
    }
    pub fn car(&self) -> Value {
        match self {
            Value::Cons(c) => c.car.clone(),
            _ => Value::Nil,
        }
    }
    pub fn cdr(&self) -> Value {
        match self {
            Value::Cons(c) => c.cdr.clone(),
            _ => Value::Nil,
        }
    }
    /// The elements of a list (a dotted tail is left out).
    pub fn iter(&self) -> ListIter {
        ListIter { next: self.clone() }
    }
    pub fn to_vec(&self) -> Vec<Value> {
        self.iter().collect()
    }
    /// A proper list (nil or cons cells ending in nil).
    pub fn is_list(&self) -> bool {
        let mut v = self.clone();
        loop {
            match v {
                Value::Nil => return true,
                Value::Cons(c) => v = c.cdr.clone(),
                _ => return false,
            }
        }
    }
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(f64::from(*i)),
            Value::Real(r) => Some(*r),
            _ => None,
        }
    }
    pub fn as_sym(&self) -> Option<&Sym> {
        match self {
            Value::Sym(s) => Some(s),
            _ => None,
        }
    }
    /// The type name `type` returns (nil for nil).
    pub fn type_name(&self) -> Option<&'static str> {
        Some(match self {
            Value::Nil => return None,
            Value::Int(_) => "INT",
            Value::Real(_) => "REAL",
            Value::Str(_) => "STR",
            Value::Sym(_) => "SYM",
            Value::Cons(_) => "LIST",
            Value::Ename(_) => "ENAME",
            Value::PickSet(_) => "PICKSET",
            Value::File(_) => "FILE",
            Value::Subr(_) => "SUBR",
            Value::Func(_) => "USUBR",
        })
    }
    /// A 2D or 3D point given as a list of two or three numbers.
    pub fn as_point(&self) -> Option<[f64; 3]> {
        let mut it = self.iter();
        let x = it.next()?.as_number()?;
        let y = it.next()?.as_number()?;
        let z = match it.next() {
            Some(v) => v.as_number()?,
            None => 0.0,
        };
        (it.next().is_none() && self.is_list()).then_some([x, y, z])
    }
}

pub struct ListIter {
    next: Value,
}

impl Iterator for ListIter {
    type Item = Value;
    fn next(&mut self) -> Option<Value> {
        match std::mem::take(&mut self.next) {
            Value::Cons(c) => {
                self.next = c.cdr.clone();
                Some(c.car.clone())
            }
            _ => None,
        }
    }
}

/// Deepest nesting `equal`, printing and reading follow before giving up (hostile input).
pub const MAX_DEPTH: usize = 2_000;

/// `eq`: the same atom or the same cons cell.
pub fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Cons(x), Value::Cons(y)) => Arc::ptr_eq(x, y),
        (Value::Str(x), Value::Str(y)) => Arc::ptr_eq(x, y) || x == y,
        (Value::PickSet(x), Value::PickSet(y)) => x.id == y.id,
        (Value::Func(x), Value::Func(y)) => Arc::ptr_eq(x, y),
        (Value::Subr(x), Value::Subr(y)) => std::ptr::eq(*x, *y),
        _ => atom_equal(a, b, 0.0),
    }
}

/// `equal`: the same structure, numbers within `fuzz`.
pub fn equal(a: &Value, b: &Value, fuzz: f64) -> bool {
    equal_depth(a, b, fuzz, 0)
}

fn equal_depth(a: &Value, b: &Value, fuzz: f64, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    let (mut a, mut b) = (a.clone(), b.clone());
    loop {
        match (&a, &b) {
            (Value::Cons(x), Value::Cons(y)) => {
                if !equal_depth(&x.car, &y.car, fuzz, depth + 1) {
                    return false;
                }
                let (na, nb) = (x.cdr.clone(), y.cdr.clone());
                a = na;
                b = nb;
            }
            _ => return atom_equal(&a, &b, fuzz),
        }
    }
}

fn atom_equal(a: &Value, b: &Value, fuzz: f64) -> bool {
    match (a, b) {
        (Value::Nil, Value::Nil) => true,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Sym(x), Value::Sym(y)) => x == y,
        (Value::Ename(x), Value::Ename(y)) => x == y,
        (Value::File(x), Value::File(y)) => x == y,
        (Value::PickSet(x), Value::PickSet(y)) => x.id == y.id,
        (Value::Subr(x), Value::Subr(y)) => std::ptr::eq(*x, *y),
        (Value::Func(x), Value::Func(y)) => Arc::ptr_eq(x, y),
        (x, y) => match (x.as_number(), y.as_number()) {
            (Some(p), Some(q)) if matches!(x, Value::Real(_)) || matches!(y, Value::Real(_)) || fuzz > 0.0 => (p - q).abs() <= fuzz,
            _ => false,
        },
    }
}

/// A real as AutoLISP prints it: six significant digits, always with a decimal point.
pub fn real_text(r: f64) -> String {
    if !r.is_finite() {
        return if r.is_nan() {
            "-1.#IND".into()
        } else if r > 0.0 {
            "1.#INF".into()
        } else {
            "-1.#INF".into()
        };
    }
    if r == 0.0 {
        return "0.0".into();
    }
    let exp = r.abs().log10().floor() as i32;
    // As C's %g with six digits: exponent form outside 1e-5 ..< 1e6.
    if !(-5..6).contains(&exp) {
        // Exponent form: d.ddddde+NNN
        let m = r / 10f64.powi(exp);
        let mut s = trim_fraction(&format!("{m:.5}"));
        if !s.contains('.') {
            s.push_str(".0");
        }
        return format!("{s}e{}{:03}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let decimals = (5 - exp).max(0) as usize;
    let mut s = trim_fraction(&format!("{r:.decimals$}"));
    if !s.contains('.') {
        s.push_str(".0");
    }
    s
}

fn trim_fraction(s: &str) -> String {
    if s.contains('.') {
        let t = s.trim_end_matches('0');
        t.strip_suffix('.').map_or(t.to_string(), |u| format!("{u}.0"))
    } else {
        s.to_string()
    }
}

/// `prin1`/`print` text (`escape`: strings quoted with their escapes) or `princ` text.
pub fn to_text(v: &Value, escape: bool) -> String {
    let mut out = String::new();
    write_value(&mut out, v, escape, 0);
    out
}

fn write_value(out: &mut String, v: &Value, escape: bool, depth: usize) {
    if depth > MAX_DEPTH {
        out.push_str("...");
        return;
    }
    match v {
        Value::Nil => out.push_str("nil"),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Real(r) => out.push_str(&real_text(*r)),
        Value::Str(s) => {
            if escape {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\t' => out.push_str("\\t"),
                        '\r' => out.push_str("\\r"),
                        '\u{1b}' => out.push_str("\\e"),
                        c => out.push(c),
                    }
                }
                out.push('"');
            } else {
                out.push_str(s);
            }
        }
        Value::Sym(s) => out.push_str(s.name()),
        Value::Cons(_) => {
            // (quote x) prints as 'x.
            if let (Value::Sym(s), Value::Cons(rest)) = (v.car(), v.cdr())
                && s.name() == "QUOTE"
                && rest.cdr.is_nil()
            {
                out.push('\'');
                write_value(out, &rest.car, escape, depth + 1);
                return;
            }
            out.push('(');
            let mut cur = v.clone();
            let mut first = true;
            let mut count = 0usize;
            loop {
                match cur {
                    Value::Cons(c) => {
                        if !first {
                            out.push(' ');
                        }
                        first = false;
                        write_value(out, &c.car, escape, depth + 1);
                        cur = c.cdr.clone();
                        count += 1;
                        if count > 1_000_000 {
                            out.push_str(" ...");
                            break;
                        }
                    }
                    Value::Nil => break,
                    other => {
                        out.push_str(" . ");
                        write_value(out, &other, escape, depth + 1);
                        break;
                    }
                }
            }
            out.push(')');
        }
        Value::Ename(h) => out.push_str(&format!("<Entity name: {:x}>", h.0)),
        Value::PickSet(p) => out.push_str(&format!("<Selection set: {:x}>", p.id)),
        Value::File(f) => out.push_str(&format!("#<FILE {f:x}>")),
        Value::Subr(b) => out.push_str(&format!("#<SUBR @{} {}>", "0", b.name)),
        Value::Func(f) => out.push_str(&format!("#<USUBR {}>", f.name.as_ref().map_or("-lambda-", |s| s.name()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_print_with_six_significant_digits() {
        assert_eq!(real_text(1.0), "1.0");
        assert_eq!(real_text(std::f64::consts::PI), "3.14159");
        assert_eq!(real_text(1.23456789), "1.23457");
        assert_eq!(real_text(0.1), "0.1");
        assert_eq!(real_text(12.5), "12.5");
        assert_eq!(real_text(-2.0), "-2.0");
        assert_eq!(real_text(100000.0), "100000.0");
        assert_eq!(real_text(123456.7), "123457.0");
        assert_eq!(real_text(0.0001), "0.0001");
        assert_eq!(real_text(1_000_000.0), "1.0e+006");
        assert_eq!(real_text(0.000001), "1.0e-006");
    }

    #[test]
    fn a_long_list_drops_without_overflowing_the_stack() {
        let mut l = Value::Nil;
        for i in 0..2_000_000 {
            l = Value::cons(Value::Int(i), l);
        }
        drop(l);
    }

    #[test]
    fn printing_and_equality() {
        let l = Value::list(vec![Value::Int(1), Value::str("a\"b"), Value::cons(Value::Int(8), Value::str("0"))]);
        assert_eq!(to_text(&l, true), "(1 \"a\\\"b\" (8 . \"0\"))");
        assert_eq!(to_text(&l, false), "(1 a\"b (8 . 0))");
        assert!(equal(&l, &l.clone(), 0.0));
        assert!(equal(&Value::Real(1.0), &Value::Real(1.0000001), 1e-3));
        assert!(!equal(&Value::Real(1.0), &Value::Real(1.1), 1e-3));
        assert!(eq(&Value::sym("a"), &Value::sym("A")));
    }
}
