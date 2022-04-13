use crate::anvill::AnvillData;
use crate::ghidra::GhidraData;

pub enum SymbolFlag {
    Function,
    Object,
}

pub struct Symbol {
    name: String,
    //section: Option<&str>,
    value: u64,
    flags: SymbolFlag,
}

impl Symbol {
    pub fn objcopy_cmd(&self) -> String {
        let flags = match self.flags {
            SymbolFlag::Function => "function",
            SymbolFlag::Object => "object",
        };
        format!("{}=0x{:08x},{}", self.name, self.value, flags)
    }
}

pub struct Symbols(pub Vec<Symbol>);

impl Symbols {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_ghidra(&mut self, ghidra_data: &GhidraData) {
        for (&addr, func) in &ghidra_data.fn_map {
            self.0.push(Symbol {
                name: func.name.to_string(),
                value: addr,
                flags: SymbolFlag::Function,
            });
        }
    }

    pub fn add_anvill(&mut self, anvill_data: &AnvillData) {
        for (&addr, var) in &anvill_data.var_map {
            if let Some(name) = var.name {
                self.0.push(Symbol {
                    name: name.to_string(),
                    value: addr,
                    flags: SymbolFlag::Object,
                });
            }
        }

        for (&addr, func) in &anvill_data.fn_map {
            if let Some(name) = func.name {
                self.0.push(Symbol {
                    name: name.to_string(),
                    value: addr,
                    flags: SymbolFlag::Function,
                });
            }
        }
    }
}
