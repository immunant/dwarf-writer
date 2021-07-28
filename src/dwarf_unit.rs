use crate::anvill::AnvillData;
use crate::dwarf_attr::name_as_bytes;
use crate::dwarf_entry::EntryRef;
use crate::elf::ELF;
use crate::types::TypeMap;
use gimli::constants;
use gimli::constants::*;
use gimli::write::{LineProgram, Unit};
use gimli::{Encoding, Format};
use object::Object;
use std::collections::HashMap;

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
    };
}

/// Creates a type map from existing DWARF debug info.
pub fn create_type_map(elf: &ELF) -> TypeMap {
    let mut type_map = HashMap::new();
    if elf.dwarf.units.count() == 0 {
        // Return an empty `TypeMap` if no debug info exists
        type_map
    } else {
        let unit_id = elf.dwarf.units.id(0);
        let unit = elf.dwarf.units.get(unit_id);

        let root = unit.get(unit.root());
        // TODO: This assumes all types are children of the root
        for &child in root.children() {
            let entry = unit.get(child);
            // Add type entries to the type map
            match entry.tag() {
                constants::DW_TAG_base_type => {
                    if let Some(name_attr) = entry.get(DW_AT_name) {
                        let name = name_as_bytes(name_attr, &elf.dwarf.strings).to_vec();

                        // Insert the type entry Id indexed by the canonical type name into the map
                        type_map.insert(name.into(), child);
                    };
                },
                constants::DW_TAG_pointer_type => {
                    // TODO: Handle this and the other missing cases
                },
                _ => {},
            }
        }
        type_map
    }
}

/// Write the anvill data as DWARF debug info and updates the type map with new
/// type entries.
pub fn process_anvill(elf: &mut ELF, mut anvill: AnvillData, type_map: &mut TypeMap) {
    create_unit_if_needed(elf);

    let dwarf = &mut elf.dwarf;
    let unit_id = dwarf.units.id(0);
    let unit = dwarf.units.get_mut(unit_id);

    let root_entry = unit.get(unit.root());
    println!(
        "Processing root entry {:?}",
        root_entry.tag().static_string()
    );

    // Get first-generation children before adding new anvill types to avoid
    // processing them
    let mut children: Vec<_> = root_entry.children().cloned().collect();

    // Add an entry for each anvill type that isn't already in the map
    for ty in &anvill.types {
        if !type_map.contains_key(&ty.name()) {
            // Create an entry for the new type
            let new_ty = unit.add(unit.root(), DW_TAG_base_type);
            let mut entry_ref = EntryRef::new(unit, new_ty, &dwarf.strings);
            entry_ref.create_type(ty);

            // Update the type map
            type_map.insert(ty.name(), new_ty);
        }
    }

    // Iterate through all entries reachable from the root
    while !children.is_empty() {
        let current_generation: Vec<_> = children.drain(..).collect();
        for entry_id in current_generation {
            let entry = unit.get(entry_id);

            // Collect the grandchildren before mutating to avoid processing newly created
            // entries
            let mut grandchildren = entry.children().cloned().collect();
            children.append(&mut grandchildren);

            // Process an entry
            println!("Processing entry {:?}", entry.tag().static_string());
            if entry.tag() == DW_TAG_subprogram {
                let mut entry_ref = EntryRef::new(unit, entry_id, &dwarf.strings);
                // This pops the given function from the anvill data if it exists
                entry_ref.update_fn(&mut anvill.fn_map, &type_map);
            }
        }
    }

    // Add a subprogram entry for each remaining anvill function
    let remaining_fn_addrs: Vec<_> = anvill.fn_map.keys().cloned().collect();
    for addr in remaining_fn_addrs {
        let fn_id = unit.add(unit.root(), DW_TAG_subprogram);
        let mut entry_ref = EntryRef::new(unit, fn_id, &dwarf.strings);
        entry_ref.create_fn(addr, &mut anvill.fn_map, &type_map);
    }
    assert!(anvill.fn_map.is_empty());
}
