use anvill::AnvillHints;
use anyhow::Result;
use dwarf_die::DIERef;
use elf::ELF;
use gimli::constants::*;
use gimli::write::{LineProgram, Unit};
use gimli::{Encoding, Format};
use object::Object;
use std::path::PathBuf;
use structopt::StructOpt;

mod anvill;
mod dwarf_attr;
mod dwarf_die;
mod elf;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(short = "b", long = "bin_in", parse(from_os_str))]
    binary_path: PathBuf,
    #[structopt(short = "a", long = "anvill", parse(from_os_str))]
    anvill_path: Option<PathBuf>,
    #[structopt(short = "m", long = "mindsight", parse(from_os_str))]
    mindsight_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let binary_path = opt.binary_path;
    let anvill_path = opt.anvill_path;
    let anvill_hints = if let Some(path) = anvill_path {
        Some(AnvillHints::new(path)?)
    } else {
        None
    };
    //let anvill_hints = anvill_path.map(|path| AnvillHints::new(path));
    // The `update_fn` pass will remove entries from this map then the `create_fn`
    // will create DIEs for the remaining entries
    let (mut anvill_fn_map, mut anvill_types) = if let Some(hint) = anvill_hints.as_ref() {
        (Some(hint.functions()), Some(hint.types()))
    } else {
        (None, None)
    };

    let mut elf = ELF::new(&binary_path)?;

    let updated_sections = elf.with_dwarf_mut(|elf, dwarf| {
        if dwarf.units.count() == 0 {
            let format = if elf.is_64() {
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
        // TODO: How should DWARF version be handled here?
        for idx in 0..dwarf.units.count() {
            let unit_id = dwarf.units.id(idx);
            let unit = dwarf.units.get_mut(unit_id);

            // process root DIE
            let root_die = unit.get(unit.root());
            println!("Processing root DIE {:?}", root_die.tag().static_string());

            let mut children = root_die.children().cloned().collect::<Vec<_>>();
            // Add DIEs for types that don't already exist
            if let Some(anvill_types) = &anvill_types {
                for ty in anvill_types {
                    let mut type_found = false;
                    for &child_id in &children {
                        let child_die = unit.get(child_id);
                        let tag = child_die.tag();
                        if tag == DW_TAG_base_type || tag == DW_TAG_pointer_type {
                            let die_ref = DIERef::new(unit, child_id, &dwarf.strings);
                            if die_ref.type_matches(ty) {
                                type_found = true;
                            }
                        }
                    }
                    if !type_found {
                        // TODO: Handle pointer_type
                        let ty_id = unit.add(unit.root(), DW_TAG_base_type);
                        let mut die_ref = DIERef::new(unit, ty_id, &dwarf.strings);
                        die_ref.create_type(ty);
                    }
                }
            }
            while !children.is_empty() {
                for die_id in children.drain(..).collect::<Vec<_>>() {
                    let die = unit.get(die_id);

                    // collect grandchildren processing before mutating the DIE since newly created
                    // DIEs should not be processed
                    let mut grandchildren = die.children().cloned().collect();
                    children.append(&mut grandchildren);

                    // process DIE
                    println!("Processing DIE {:?}", die.tag().static_string());
                    if die.tag() == DW_TAG_subprogram {
                        if let Some(fn_map) = &mut anvill_fn_map {
                            let mut die_ref = DIERef::new(unit, die_id, &dwarf.strings);
                            die_ref.update_fn(fn_map);
                        }
                    }
                }
            }
            // Add a subprogram DIE for each remaining `fn_map` entry
            if let Some(fn_map) = &mut anvill_fn_map {
                let remaining_entries = fn_map.keys().cloned().collect::<Vec<_>>();
                for addr in remaining_entries {
                    let fn_id = unit.add(unit.root(), DW_TAG_subprogram);
                    let mut die_ref = DIERef::new(unit, fn_id, &dwarf.strings);
                    die_ref.create_fn(addr, fn_map);
                }
            }
        }
        Ok(())
    })?;

    ELF::write_sections(&updated_sections)
}
