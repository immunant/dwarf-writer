use crate::anvill::AnvillData;
use crate::dwarf_attr::name_as_bytes;
use crate::dwarf_entry::EntryRef;
use crate::elf::ELF;
use crate::types::TypeMap;
use gimli::constants;
use gimli::constants::*;
use gimli::write::{LineProgram, Unit, UnitEntryId};
use gimli::{Encoding, Format};
use object::Object;
use std::collections::HashMap;

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

pub fn create_type_map(elf: &ELF) -> TypeMap {
    let mut type_map = HashMap::new();
    if elf.dwarf.units.count() == 0 {
        type_map
    } else {
        let unit_id = elf.dwarf.units.id(0);
        let unit = elf.dwarf.units.get(unit_id);

        let root = unit.get(unit.root());
        for &child in root.children() {
            let entry = unit.get(child);
            // TODO: Assumes all types are children of the root
            match entry.tag() {
                constants::DW_TAG_base_type => {
                    if let Some(name_attr) = entry.get(DW_AT_name) {
                        let name = name_as_bytes(name_attr, &elf.dwarf.strings).to_vec();
                        type_map.insert(name.into(), child);
                    };
                },
                constants::DW_TAG_pointer_type => {
                    // TODO: Handle this and other missing cases
                },
                _ => {},
            }
        }
        type_map
    }
}

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

    // Get entry IDs for root entry's children
    let mut children = root_entry.children().cloned().collect::<Vec<_>>();

    // Add an entry for each type that doesn't already exist
    for ty in &anvill.types {
        if !type_map.contains_key(&ty.name()) {}

        /*
        let mut type_found = false;
        //&
        //type_map.contains_key(ty.into().to_vec());


        for &child_id in &children {
            let child_entry = unit.get(child_id);
            let tag = child_entry.tag();
            if tag == DW_TAG_base_type || tag == DW_TAG_pointer_type {
                let entry_ref = EntryRef::new(unit, child_id, &dwarf.strings);
                if entry_ref.type_matches(ty) {
                    type_found = true;
                    break
                }
            }
        }
        if !type_found {
            // TODO: Handle DW_TAG_pointer_type
            let ty_id = unit.add(unit.root(), DW_TAG_base_type);
            let mut entry_ref = EntryRef::new(unit, ty_id, &dwarf.strings);
            entry_ref.create_type(ty);
        }
        */
    }
    while !children.is_empty() {
        for entry_id in children.drain(..).collect::<Vec<_>>() {
            let entry = unit.get(entry_id);

            // Collect grandchildren before mutating the entry since newly created entries
            // should not be processed
            let mut grandchildren = entry.children().cloned().collect();
            children.append(&mut grandchildren);

            // Process entry
            println!("Processing entry {:?}", entry.tag().static_string());
            if entry.tag() == DW_TAG_subprogram {
                let mut entry_ref = EntryRef::new(unit, entry_id, &dwarf.strings);
                // This pops the given function from the map if it exists
                entry_ref.update_fn(&mut anvill.fn_map);
            }
        }
    }
    // Add a subprogram entry for each remaining `fn_map` entry
    let remaining_entries = anvill.fn_map.keys().cloned().collect::<Vec<_>>();
    for addr in remaining_entries {
        let fn_id = unit.add(unit.root(), DW_TAG_subprogram);
        let mut entry_ref = EntryRef::new(unit, fn_id, &dwarf.strings);
        entry_ref.create_fn(addr, &mut anvill.fn_map);
    }
}
