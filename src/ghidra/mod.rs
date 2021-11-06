use crate::functions::{DwarfFunction, FnMap, Parameter, Provided};
use crate::types::{CanonicalTypeName, DwarfType};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

impl GhidraInput {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let mut hints = csv::Reader::from_reader(reader);
        let mut functions = Vec::new();
        for h in hints.deserialize() {
            functions.push(h?);
        }
        Ok(GhidraInput { functions })
    }

    pub fn as_map(&self) -> Result<FnMap> {
        let mut res = HashMap::with_capacity(self.functions.len());
        for f in &self.functions {
            let low_pc = u64::from_str_radix(&f.location, 16)?;
            let hi_pc = u64::from_str_radix(&f.size, 16)? + low_pc;
            let (return_values, parameters) = Self::parse_signature(&f.signature);
            let dwarf_function = DwarfFunction {
                name: Provided::Value(&f.name),
                hi_pc: Provided::Value(hi_pc),
                return_values,
                parameters,
                ..Default::default()
            };
            res.insert(low_pc, dwarf_function);
        }
        Ok(res)
    }

    /// Returns a tuple of (return_types, parameters). Ghidra currently only
    /// provides a single return value, but it's inserted into a vector to
    /// simplify the transformation to a `DwarfFunction`.
    fn parse_signature(fn_sig: &str) -> (Provided<Vec<DwarfType>>, Provided<Vec<Parameter>>) {
        let mut sig_iter = fn_sig.split("(");
        let left_str = sig_iter.next().unwrap();
        let right_str = sig_iter.next().unwrap();

        let mut left_iter = left_str.rsplit(' ');
        let fn_name_str = left_iter.next().unwrap();
        let ret_str = left_iter.rfold(String::new(), |mut acc, s| {
            acc.push(' ');
            acc.push_str(s);
            acc
        });
        let right_iter = right_str.split(',').map(|ty| &ty[..ty.len() - 1]);
        let mut params = Vec::new();
        for p in right_iter {
            if p == "void" || p == "" {
                break
            } else {
                let mut param_iter = p.rsplit(' ');
                let name = Provided::Value(param_iter.next().unwrap());
                let ty_name = param_iter.rfold(String::new(), |mut acc, s| {
                    acc.push(' ');
                    acc.push_str(s);
                    acc
                });
                let param = Parameter {
                    name,
                    location: Provided::Unavailable,
                    ty: Provided::Value(Self::parse_type(&ty_name)),
                };
                params.push(param);
            }
        }
        let params_provided = if params.len() == 0 {
            Provided::Nothing
        } else {
            Provided::Value(params)
        };
        let ret_ty = Self::parse_type(&ret_str);
        let ret = Provided::Value(vec![ret_ty]);
        (ret, params_provided)
    }

    fn parse_type(ty: &str) -> DwarfType {
        let ty = ty.trim_end();
        match ty.strip_suffix("*") {
            Some(inner_ty) => DwarfType::new_pointer(Self::parse_type(inner_ty)),
            None => DwarfType::new_primitive(
                CanonicalTypeName::from(ty.trim_start().as_bytes().to_vec()),
                None,
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GhidraInput {
    functions: Vec<Function>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Function {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Function Size")]
    size: String,
    #[serde(rename = "Location")]
    location: String,
    #[serde(rename = "Function Signature")]
    signature: String,
}
