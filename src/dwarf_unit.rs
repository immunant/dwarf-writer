use crate::anvill::AnvillData;
use crate::dwarf_attr::name_as_bytes;
use crate::dwarf_entry::EntryRef;
use crate::elf::ELF;
use crate::types::{DwarfType, TypeMap};
use gimli::constants;
use gimli::constants::*;
use gimli::write::{Dwarf, LineProgram, Unit, UnitId, UnitEntryId};
use gimli::{Encoding, Format};
use log::trace;
use object::Object;
use std::collections::HashMap;

pub struct UnitRef<'a> {
    elf: &'a ELF,
    // The unit's ID.
    id: UnitId,
}

/// Creates a DWARF unit if none exists in the `ELF`.
fn create_unit_if_needed(elf: &mut ELF) {
    let is_64_bit = elf.object().is_64();
    let dwarf = &mut elf.dwarf;
    if dwarf.units.count() == 0 {
        let format = if is_64_bit {
            Format::Dwarf64
        } else {
            Format::Dwarf32
        };
        let encoding = Encoding {
            address_size: format.word_size(),
            format,
            // TODO: Make this configurable
            version: 4,
        };
        let line_program = LineProgram::none();
        let root = Unit::new(encoding, line_program);
        dwarf.units.add(root);
    }
}

/// Creates a type map from existing DWARF debug info. Returns an empty map if
/// no debug info exists.
pub fn create_type_map(dwarf: &Dwarf) -> TypeMap {
    let mut type_map = HashMap::new();

    // Return an empty `TypeMap` if no debug info exists
    if dwarf.units.count() != 0 {
        let unit_id = dwarf.units.id(0);
        let unit = dwarf.units.get(unit_id);

        let root = unit.get(unit.root());
        // TODO: This assumes all types are children of the root
        for &child in root.children() {
            let entry = unit.get(child);
            // Add type entries to the type map
            match entry.tag() {
                constants::DW_TAG_base_type => {
                    if let Some(name_attr) = entry.get(DW_AT_name) {
                        let name = name_as_bytes(name_attr, &dwarf.strings).to_vec();

                        // Insert the type entry Id indexed by the canonical type name into the map
                        type_map.insert(DwarfType::new(name.into()), child);
                    };
                },
                constants::DW_TAG_pointer_type => {
                    // TODO: Handle this and the other missing cases
                },
                _ => {},
            }
        }
    };
    type_map
}

/// Write the anvill data as DWARF debug info and updates the type map with new
/// type entries.
pub fn process_anvill(elf: &mut ELF, mut anvill: AnvillData, type_map: &mut TypeMap) {
    create_unit_if_needed(elf);

    fn unit(elf: &ELF) -> &Unit {
        let id = elf.dwarf.units.id(0);
        elf.dwarf.units.get(id)
    }

    fn mut_unit(elf: &mut ELF) -> &mut Unit {
        let id = elf.dwarf.units.id(0);
        elf.dwarf.units.get_mut(id)
    }

    // Get IDs first-generation children
    let unit = unit(elf);
    let root_id = unit.root();
    let root_entry = unit.get(root_id);
    let mut children: Vec<_> = root_entry.children().cloned().collect();

    // Add a child entry to the root for each type that isn't already in the map
    for ty in anvill.types {
        if !type_map.contains_key(&ty) {
            let mut ty_entry = new_entry(&mut elf.dwarf, root_id, ty.tag());
            ty_entry.init_type(&ty, type_map);

            // Update the type map with the new type
            type_map.insert(ty, ty_entry.id());
        }
    }

    // Iterate through the children excluding the newly added types
    while !children.is_empty() {
        let current_generation: Vec<_> = children.drain(..).collect();
        for entry_id in current_generation {
            let unit = mut_unit(elf);
            let entry = unit.get(entry_id);

            // Get the next generation children before mutating to avoid reprocessing newly
            // created entries
            let mut next_generation = entry.children().cloned().collect();
            children.append(&mut next_generation);

            match entry.tag() {
                constants::DW_TAG_subprogram => {
                    let mut fn_entry = EntryRef::new(&mut elf.dwarf, entry_id);

                    // This removes the given function from the anvill data if it exists
                    fn_entry.update_fn(&mut anvill.fn_map, &type_map);
                },
                _ => (),
            }
        }
    }

    // Add a subprogram entry for each remaining function
    let remaining_fn_addrs: Vec<_> = anvill.fn_map.keys().cloned().collect();
    for addr in remaining_fn_addrs {
        let mut fn_entry = new_entry(&mut elf.dwarf, root_id, DW_TAG_subprogram);
        fn_entry.init_fn(addr, &mut anvill.fn_map, &type_map);
    }
    assert!(anvill.fn_map.is_empty());
}

fn new_entry(dwarf: &mut Dwarf, parent: UnitEntryId, tag: DwTag) -> EntryRef {
    let unit_id = dwarf.units.id(0);
    let unit = dwarf.units.get_mut(unit_id);

    let id = unit.add(parent, tag);
    EntryRef::new(dwarf, id)
}
