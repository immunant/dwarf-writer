use crate::anvill;
use crate::anvill::AnvillFnMap;
use crate::dwarf_attr::*;
use gimli::constants;
use gimli::constants::*;
use gimli::write::{Address, AttributeValue, StringTable, Unit, UnitEntryId};

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
#[derive(Debug)]
pub struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut Unit,
    // The DIE's ID
    self_id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a StringTable,
    //types: &'a TypeMap,
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

    /// Updates an existing function's subprogram DIE.
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
        let start_address = low_pc_to_u64(low_pc_attr);

        // Search for this function's anvill data by start address
        let fn_data = anvill_data.remove(&start_address);
        if let Some(fn_data) = fn_data {
            // Update function name overwriting any existing DW_AT_name
            let existing_name = die.get(DW_AT_name);
            let new_name = match (existing_name, fn_data.name) {
                (None, None) => Some(format!("__unnamed_fn_{}", start_address)),
                (Some(_), None) => None,
                (_, Some(name)) => Some(name.to_string()),
            };
            if let Some(name) = new_name {
                let die = unit.get_mut(self_id);
                die.set(DW_AT_name, AttributeValue::String(name.as_bytes().to_vec()));
            }

            // Update function parameters
            if let Some(parameters) = fn_data.func.parameters() {
                for param in parameters {
                    // Search for an existing param DIE by location
                    let die = unit.get(self_id);
                    let existing_die_id = die.children().find(|&&child_id| {
                        let child_die = unit.get(child_id);
                        let child_tag = child_die.tag();
                        let location_attr = child_die
                            .get(DW_AT_location)
                            .expect("No DW_AT_location found in DW_TAG_formal_parameter DIE");
                        let param_location = dwarf_location(&param.location());
                        child_tag == DW_TAG_formal_parameter && *location_attr == param_location
                    });

                    // Add a formal parameter DIE if an existing DIE wasn't found
                    let param_id = match existing_die_id {
                        Some(id) => *id,
                        None => {
                            let id = unit.add(self_id, DW_TAG_formal_parameter);
                            unit.get_mut(id)
                                .set(DW_AT_location, dwarf_location(&param.location()));
                            id
                        },
                    };
                    let param_die = unit.get_mut(param_id);
                    if let Some(param_name) = param.name() {
                        param_die.set(
                            DW_AT_name,
                            AttributeValue::String(param_name.as_bytes().to_vec()),
                        );
                    };
                }
                // Mark the subprogram DIE as prototyped
                let die = unit.get_mut(self_id);
                die.set(DW_AT_prototyped, AttributeValue::Flag(true));
            }
        }
    }

    /// Checks if the given anvill type matches the type DIE.
    ///
    /// A type may have various string representations (e.g. `bool` from
    /// `stdbool.h` expands to `_Bool` while C++/rust's boolean is `bool`).
    /// meaning this method may produce false negatives. In the case of a
    /// false negative, a new type DIE will be created for the incorrectly
    /// identified type, but subsequent comparisons between the new DIE and
    /// the type will always succeed.
    pub fn type_matches(&self, ty: &anvill::Type) -> bool {
        let die = self.unit.get(self.self_id);
        match die.tag() {
            constants::DW_TAG_base_type => match die.get(DW_AT_name) {
                Some(name_attr) => {
                    if let Some(name) = name_to_anvill_ty(name_attr, self.strings) {
                        name == *ty
                    } else {
                        false
                    }
                },
                None => false,
            },
            constants::DW_TAG_pointer_type => false,
            _ => false,
        }
    }

    pub fn create_type(&mut self, ty: &anvill::Type) {
        let die = self.unit.get_mut(self.self_id);
        let ty_name: &[u8] = ty.into();
        die.set(DW_AT_name, AttributeValue::String(ty_name.to_vec()));
        // TODO: DW_AT_encoding
        die.set(DW_AT_byte_size, AttributeValue::Data1(ty.size()));
    }
}
