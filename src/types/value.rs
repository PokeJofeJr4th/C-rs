use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::Display,
    hash::Hash,
    ops::{Add, BitAnd, BitOr, Div, Mul, Neg, Rem, Sub},
    rc::Rc,
};

use super::{Pointer, Syntax};

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Boolean {
    True,
    False,
    Maybe,
}

impl Display for Boolean {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Maybe => write!(f, "maybe"),
        }
    }
}

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
#[repr(C)]
pub enum Value {
    Boolean(Boolean),
    String(Rc<str>),
    Number(f64),
    Object(BTreeMap<Value, Pointer>),
    Function(Vec<Rc<str>>, Syntax),
    Keyword(Keyword),
}

impl Eq for Value {}

impl Default for Value {
    fn default() -> Self {
        Self::Object(BTreeMap::new())
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match unsafe {
            core::mem::transmute::<_, u64>(core::mem::discriminant(self)).cmp(
                &core::mem::transmute::<_, u64>(core::mem::discriminant(other)),
            )
        } {
            Ordering::Equal => {}
            other => return Some(other),
        }
        match (self, other) {
            (Self::Number(lhs), Self::Number(rhs)) => lhs.partial_cmp(rhs),
            (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
            (Self::Boolean(lhs), Self::Boolean(rhs)) => lhs.partial_cmp(rhs),
            (Self::Keyword(lhs), Self::Keyword(rhs)) => lhs.partial_cmp(rhs),
            _ => todo!(),
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        if let Some(ord) = self.partial_cmp(other) {
            return ord;
        };
        todo!()
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean(b) => write!(f, "{b}"),
            Self::String(str) => write!(f, "{str:?}"),
            Self::Number(num) => write!(f, "{num}"),
            Self::Object(obj) => {
                let mut map = f.debug_struct("object");
                for (k, v) in obj {
                    map.field(&format!("{k}"), &format!("{v}"));
                }
                map.finish()
            }
            Self::Function(args, body) => {
                write!(f, "{args:?} -> {body}")
            }
            Self::Keyword(kw) => write!(f, "{kw}"),
        }
    }
}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Boolean(bool) => bool.hash(state),
            Self::String(str) => str.hash(state),
            Self::Number(float) => (*float).to_bits().hash(state),
            Self::Object(obj) => {
                let mut vec: Vec<_> = obj.iter().collect::<Vec<_>>();
                vec.sort_by_key(|&(k, _)| k);
                for (k, v) in vec {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Self::Function(inputs, content) => {
                inputs.hash(state);
                content.hash(state);
            }
            Self::Keyword(keyword) => keyword.hash(state),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(Boolean::from(value))
    }
}

impl Add for Value {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(lhs), Self::Number(rhs)) => Self::Number(lhs + rhs),
            (Self::Boolean(bool), Self::Number(num)) | (Self::Number(num), Self::Boolean(bool)) => {
                Self::Number(
                    match bool {
                        Boolean::False => 0.0,
                        Boolean::Maybe => 0.5,
                        Boolean::True => 1.0,
                    } + num,
                )
            }
            (Self::String(lhs), rhs) => {
                Self::String((String::from(&*lhs) + &rhs.to_string()).into())
            }
            _ => Self::default(),
        }
    }
}

impl Sub for Value {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(lhs), Self::Number(rhs)) => Self::Number(lhs - rhs),
            _ => Self::default(),
        }
    }
}

impl Mul for Value {
    type Output = Self;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(lhs), Self::Number(rhs)) => Self::Number(lhs * rhs),
            (Self::String(str), Self::Number(num)) => {
                let mut str_buf = str.repeat(num.abs().floor() as usize);
                let portion = ((num.abs() - num.abs().floor()) * str.len() as f64) as usize;
                if portion > 0 {
                    str_buf.push_str(&str[0..portion]);
                }
                if num.is_sign_negative() {
                    str_buf = str_buf.chars().rev().collect();
                }
                Self::String(str_buf.into())
            }
            _ => Self::default(),
        }
    }
}

impl Div for Value {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(lhs), Self::Number(rhs)) => {
                if rhs == 0.0 {
                    Self::default()
                } else {
                    Self::Number(lhs / rhs)
                }
            }
            _ => Self::default(),
        }
    }
}

impl Rem for Value {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Number(lhs), Self::Number(rhs)) => {
                if rhs == 0.0 {
                    Self::default()
                } else {
                    Self::Number(lhs % rhs)
                }
            }
            _ => Self::default(),
        }
    }
}

impl Neg for Value {
    type Output = Self;
    fn neg(self) -> Self::Output {
        match self {
            Self::Boolean(Boolean::False) => Self::Boolean(Boolean::True),
            Self::Boolean(Boolean::True) => Self::Boolean(Boolean::False),
            Self::Boolean(Boolean::Maybe) => Self::Boolean(Boolean::Maybe),
            Self::Number(num) => Self::Number(-num),
            Self::String(str) => Self::String(str.chars().rev().collect::<String>().into()),
            _ => Self::default(),
        }
    }
}

impl BitAnd for Value {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self.bool(), rhs.bool()) {
            (Boolean::False, _) | (_, Boolean::False) => Self::from(false),
            (Boolean::True, Boolean::True) => Self::from(true),
            _ => Self::Boolean(Boolean::Maybe),
        }
    }
}

impl BitOr for Value {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self.bool(), rhs.bool()) {
            (Boolean::True, _) | (_, Boolean::True) => Self::from(true),
            (Boolean::False, Boolean::False) => Self::from(false),
            _ => Self::Boolean(Boolean::Maybe),
        }
    }
}

impl Value {
    pub fn eq(&self, rhs: &Self, precision: u8) -> Self {
        if precision <= 2 && self.bool() == Boolean::False && rhs.bool() == Boolean::False {
            return Self::from(true);
        }
        // true == `aaa`
        if precision == 1 {
            if let (&Self::Boolean(bool), rhs) | (rhs, &Self::Boolean(bool)) = (self, rhs) {
                if rhs.bool() == bool {
                    return Self::from(true);
                }
            }
        }
        if precision == 2 {
            return Self::from(format!("{self}") == format!("{rhs}"));
        } else if precision == 1
            && format!("{self}").to_lowercase().trim() == format!("{rhs}").to_lowercase().trim()
        {
            return Self::from(true);
        }
        match (self, rhs) {
            (&Self::Number(lhs), &Self::Number(rhs)) => {
                Self::from(lhs == rhs || (precision == 1 && (lhs / rhs).ln().abs() < 0.1))
            }
            (Self::String(lhs), Self::String(rhs)) => Self::from(*lhs == *rhs),
            (&Self::Keyword(lhs), Self::Keyword(rhs)) => Self::from(lhs == *rhs),
            (Self::String(ref str), &Self::Number(num))
            | (&Self::Number(num), Self::String(ref str)) => {
                let Ok(str_parse) = str.parse::<f64>() else {
                    return Self::from(false)
                };
                Self::from(
                    num == str_parse || (precision == 1 && (num / str_parse).ln().abs() < 0.1),
                )
            }
            (Self::Object(lhs), Self::Object(rhs)) => Self::from(
                !lhs.iter().any(|(k, v)| {
                    rhs.get(k)
                        .map_or(true, |r| r.eq(v, precision) == Self::from(false))
                }) && !rhs.iter().any(|(k, _)| lhs.get(k).is_none()),
            ),
            _ => Self::from(false),
        }
    }

    pub fn bool(&self) -> Boolean {
        match self {
            Self::Boolean(bool) => *bool,
            Self::Number(num) => {
                if *num >= 1.0 {
                    Boolean::True
                } else if *num <= 0.0 {
                    Boolean::False
                } else {
                    Boolean::Maybe
                }
            }
            Self::String(str) => {
                if str.is_empty() {
                    Boolean::False
                } else {
                    Boolean::True
                }
            }
            Self::Object(obj) => {
                if obj.is_empty() {
                    Boolean::False
                } else {
                    Boolean::True
                }
            }
            _ => Boolean::Maybe,
        }
    }

    pub const fn empty_object() -> Self {
        Self::Object(BTreeMap::new())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<Rc<str>> for Value {
    fn from(value: Rc<str>) -> Self {
        Self::String(value)
    }
}

impl From<Keyword> for Value {
    fn from(value: Keyword) -> Self {
        Self::Keyword(value)
    }
}

impl From<Boolean> for Value {
    fn from(value: Boolean) -> Self {
        Self::Boolean(value)
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum Keyword {
    Const,
    Delete,
    Eval,
    Function,
    If,
    Var,
}

impl Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const => write!(f, "const"),
            Self::Var => write!(f, "var"),
            Self::Delete => write!(f, "delete"),
            Self::Function => write!(f, "function"),
            Self::If => write!(f, "if"),
            Self::Eval => write!(f, "eval"),
        }
    }
}
