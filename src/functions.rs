use crate::types::DwarfType;
use gimli::write::Expression;
use std::collections::HashMap;

pub type FnMap<'a> = HashMap<u64, DwarfFunction<'a>>;

#[derive(Debug)]
pub enum Provided<T> {
    /// A value provided by the input data
    Value(T),
    /// The input data did not provide a value
    Nothing,
    /// The input data cannot provide a value
    Unavailable,
}

impl<T> From<Provided<T>> for Option<T> {
    fn from(provided: Provided<T>) -> Option<T> {
        match provided {
            Provided::Value(val) => Some(val),
            Provided::Nothing | Provided::Unavailable => None,
        }
    }
}

#[derive(Debug)]
pub struct DwarfFunction<'a> {
    pub name: Provided<&'a str>,
    pub hi_pc: Provided<u64>,
    pub return_address: Provided<u64>,
    pub no_return: Provided<bool>,
    pub prototyped: Provided<bool>,
    pub return_values: Provided<Vec<DwarfType>>,
    pub parameters: Provided<Vec<Parameter<'a>>>,
    pub file: Provided<&'a str>,
    pub line: Provided<u64>,
    pub local_vars: Provided<Vec<LocalVariable<'a>>>,
}

impl<'a> DwarfFunction<'a> {
    pub fn types(&self) -> Vec<DwarfType> {
        let mut res = Vec::new();
        if let Provided::Value(ret) = &self.return_values {
            for r in ret {
                res.push(r.clone());
            }
        }
        if let Provided::Value(params) = &self.parameters {
            for p in params {
                if let Provided::Value(param_ty) = &p.ty {
                    res.push(param_ty.clone());
                }
            }
        }
        res
    }
}

impl<'a> Default for DwarfFunction<'a> {
    fn default() -> DwarfFunction<'a> {
        DwarfFunction {
            name: Provided::Unavailable,
            hi_pc: Provided::Unavailable,
            return_address: Provided::Unavailable,
            no_return: Provided::Unavailable,
            prototyped: Provided::Unavailable,
            return_values: Provided::Unavailable,
            parameters: Provided::Unavailable,
            file: Provided::Unavailable,
            line: Provided::Unavailable,
            local_vars: Provided::Unavailable,
        }
    }
}

#[derive(Debug)]
pub struct Parameter<'a> {
    pub name: Provided<&'a str>,
    pub location: Provided<Expression>,
    pub ty: Provided<DwarfType>,
}

#[derive(Debug)]
pub struct LocalVariable<'a> {
    name: Provided<&'a str>,
    ty: Provided<DwarfType>,
}
