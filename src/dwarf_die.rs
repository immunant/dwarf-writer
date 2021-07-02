use crate::anvill;
use crate::anvill::AnvillFnMap;
use crate::dwarf_attr::*;
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, StringTable, Unit, UnitEntryId};

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
pub struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut Unit,
    // The DIE's ID
    self_id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a StringTable,
}

impl<'a> DIERef<'a> {
    pub fn new(unit: &'a mut Unit, self_id: UnitEntryId, strings: &'a StringTable) -> Self {
        DIERef {
            unit,
            self_id,
            strings,
        }
    }
/// Initializes a newly created subprogram DIE.
pub fn create_fn(&mut self, addr: u64, anvill_data: &mut AnvillFnMap) {
    let die = self.unit.get_mut(self.self_id);
    die.set(
        DW_AT_low_pc,
        AttributeValue::Address(Address::Constant(addr)),
    );
    self.update_fn(anvill_data)
}

/// Updates or creates a function prototype for an existing DW_TAG_subprogram
/// DIE.
pub fn update_fn(&mut self, anvill_data: &mut AnvillFnMap) {
    let DIERef {
        unit,
        self_id,
        strings,
    } = self;
    let self_id = *self_id;
    let die = unit.get(self_id);

    // Get this function's address from the existing DWARF data
    let low_pc_attr = die
        .get(DW_AT_low_pc)
        .expect("No DW_AT_low_pc found in DW_TAG_subprogram DIE");
    let low_pc = low_pc_to_u64(low_pc_attr);

    // Get the anvill data for this function
    let fn_data = anvill_data.remove(&low_pc);
    if let Some(fn_data) = fn_data {
        // Update function name overwriting any existing DW_AT_name
        if let Some(name) = fn_data.name {
            let die = unit.get_mut(self_id);
            die.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
        }
        // Update function parameters
        if let Some(parameters) = fn_data.func.parameters() {
            for param in parameters {
                println!("anvill had param {:?} for {:#x}", param, low_pc);
                // Search for a matching DIE by name
                // TODO: Search for a matching formal parameter DIE by location
                let die = unit.get(self_id);
                let matching_die_id = die.children().find(|&&child_id| {
                    let child_die = unit.get(child_id);
                    let child_tag = child_die.tag();
                    let name_attr = child_die
                        .get(DW_AT_name)
                        .expect("assume anvill json always names args for now");
                    let name = name_to_str(name_attr, strings);
                    child_tag == DW_TAG_formal_parameter && name == param.name().unwrap().as_bytes()
                });
                // Add a formal parameter DIE if a matching DIE wasn't found
                let param_id = match matching_die_id {
                    Some(&id) => id,
                    None => unit.add(self_id, DW_TAG_formal_parameter),
                };
                let param_die = unit.get_mut(param_id);
                if let Some(param_name) = param.name() {
                    param_die.set(
                        DW_AT_name,
                        AttributeValue::String(param_name.as_bytes().to_vec()),
                    );
                    //param_die.set(DW_AT_base_type,
                }
            }
            // Mark the subprogram DIE as prototyped
            let die = unit.get_mut(self_id);
            die.set(DW_AT_prototyped, AttributeValue::Flag(true));
        }
    }
}

pub fn type_matches(&self, ty: &anvill::Type) -> bool {
    todo!("implement this")
}
pub fn create_type(&mut self) {
    todo!("implement this")
}
}
