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

    pub fn data(&self) -> Result<GhidraData> {
        let mut fn_map = HashMap::new();
        for f in &self.functions {
            let low_pc = u64::from_str_radix(&f.location, 16)?;
            let high_pc = u64::from_str_radix(&f.size, 16)? + low_pc;
            let (return_ty, parameters) = Self::parse_signature(&f.signature);
            fn_map.insert(
                low_pc,
                Function {
                    low_pc,
                    high_pc,
                    return_ty,
                    parameters,
                    name: &f.name,
                },
            );
        }
        Ok(GhidraData { fn_map })
    }

    /// Returns a tuple of (return_types, parameters). Ghidra currently only
    /// provides a single return value, but it's inserted into a vector to
    /// simplify the transformation to a `DwarfFunction`.
    fn parse_signature(fn_sig: &str) -> (Option<DwarfType>, Vec<Parameter>) {
        let mut sig_iter = fn_sig.split("(");
        let left_str = sig_iter.next().unwrap();
        let right_str = sig_iter.next().unwrap();

        let mut left_iter = left_str.rsplit(' ');
        let _fn_name = left_iter.next().unwrap();
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
                let name = param_iter.next().unwrap();
                let ty_name = param_iter.rfold(String::new(), |mut acc, s| {
                    acc.push(' ');
                    acc.push_str(s);
                    acc
                });
                let param = Parameter {
                    name,
                    ty: Self::parse_type(&ty_name),
                };
                params.push(param);
            }
        }
        let ret_ty = Self::parse_type(&ret_str);
        (ret_ty, params)
    }

    fn parse_type(ty: &str) -> Option<DwarfType> {
        let ty = ty.trim_end().trim_start();
        if ty == "undefined" || ty == "thunk undefined" {
            return None
        };
        let res = match ty.strip_suffix("*") {
            Some(inner_ty) => DwarfType::new_pointer(Self::parse_type(inner_ty).unwrap()),
            None => DwarfType::new_primitive(
                CanonicalTypeName::from(ty.trim_start().as_bytes().to_vec()),
                None,
            ),
        };
        Some(res)
    }
}

pub struct GhidraData<'a> {
    pub fn_map: HashMap<u64, Function<'a>>,
}

impl<'a> GhidraData<'a> {
    pub fn types(&self) -> Vec<DwarfType> {
        let mut res = Vec::new();
        for function in self.fn_map.values() {
            for param in &function.parameters {
                if let Some(param_ty) = &param.ty {
                    res.push(param_ty.clone());
                }
            }
            if let Some(ret_ty) = &function.return_ty {
                res.push(ret_ty.clone());
            }
        }
        res
    }
}

pub struct Function<'a> {
    pub low_pc: u64,
    pub high_pc: u64,
    pub name: &'a str,
    pub return_ty: Option<DwarfType>,
    pub parameters: Vec<Parameter<'a>>,
}

pub struct Parameter<'a> {
    pub name: &'a str,
    pub ty: Option<DwarfType>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GhidraInput {
    functions: Vec<FunctionInput>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FunctionInput {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Function Size")]
    size: String,
    #[serde(rename = "Location")]
    location: String,
    #[serde(rename = "Function Signature")]
    signature: String,
}
