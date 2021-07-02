use anvill_parser::{AnvillFnMap, AnvillHints};
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::*;
use gimli::write;
use gimli::write::{EndianVec, LineProgram, Sections, StringTable, Unit, UnitEntryId};
use gimli::{Encoding, Format, RunTimeEndian};
use object::Object;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

mod anvill_parser;
mod dwarf_writer;

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

fn write_sections(sections: &Sections<EndianVec<RunTimeEndian>>) -> Result<()> {
    sections.for_each(|id, data| {
        if !data.slice().is_empty() {
            println!("Writing {} section", id.name());
            let file_name = &id.name()[1..];
            let mut file = fs::File::create(file_name)?;
            file.write_all(data.slice())?;
        }
        Ok(())
    })
}

fn name_to_str<'a>(attr: &'a write::AttributeValue, strings: &'a StringTable) -> &'a [u8] {
    // TODO: This is missing some cases
    match attr {
        write::AttributeValue::String(s) => s,
        write::AttributeValue::StringRef(str_id) => strings.get(*str_id),
        _ => panic!("Unhandled `AttributeValue` variant in `name_to_str`"),
    }
}

fn low_pc_to_u64(attr: &write::AttributeValue) -> u64 {
    // TODO: Handle Address::Symbol
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `low_pc_to_u64`"),
    }
}

fn high_pc_to_u64(attr: &write::AttributeValue) -> u64 {
    match attr {
        write::AttributeValue::Address(write::Address::Constant(addr)) => *addr,
        write::AttributeValue::Udata(addr) => *addr,
        _ => panic!("Unhandled `AttributeValue` variant in `high_pc_to_u64`"),
    }
}

// References to a subset of `gimli::write::Dwarf` to modify a specific DIE.
struct DIERef<'a> {
    // The unit containing the DIE
    unit: &'a mut write::Unit,
    // The DIE's ID
    self_id: UnitEntryId,

    // Miscellaneous DWARF info
    strings: &'a write::StringTable,
}

impl<'a> DIERef<'a> {
    pub fn new(
        unit: &'a mut write::Unit, self_id: UnitEntryId, strings: &'a write::StringTable,
    ) -> Self {
        DIERef {
            unit,
            self_id,
            strings,
        }
    }
}

/// Initializes a newly created subprogram DIE.
fn create_fn(die_ref: DIERef, addr: u64, anvill_data: &mut AnvillFnMap) {
    let die = die_ref.unit.get_mut(die_ref.self_id);
    die.set(
        DW_AT_low_pc,
        write::AttributeValue::Address(write::Address::Constant(addr)),
    );
    update_fn(die_ref, anvill_data)
}

/// Updates or creates a function prototype for an existing DW_TAG_subprogram
/// DIE.
fn update_fn(die_ref: DIERef, anvill_data: &mut AnvillFnMap) {
    let DIERef {
        unit,
        self_id,
        strings,
    } = die_ref;
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
            die.set(
                DW_AT_name,
                write::AttributeValue::String(name.as_bytes().to_vec()),
            );
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
                        write::AttributeValue::String(param_name.as_bytes().to_vec()),
                    );
                    //param_die.set(DW_AT_base_type,
                }
            }
            // Mark the subprogram DIE as prototyped
            let die = unit.get_mut(self_id);
            die.set(DW_AT_prototyped, write::AttributeValue::Flag(true));
        }
    }
}

fn type_matches(die_ref: DIERef, ty: &anvill_parser::Type) -> bool {
    todo!("implement this")
}
fn create_type(die_ref: DIERef) {
    todo!("implement this")
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
    let mut anvill_fn_map = anvill_hints.as_ref().map(|hint| hint.functions());

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
            for ty in hints.types() {
                let mut type_found = false;
                for &child_id in &children {
                    let child_die = unit.get(child_id);
                    let tag = child_die.tag();
                    if tag == DW_TAG_base_type || tag == DW_TAG_pointer_type {
                        let die_ref = DIERef::new(unit, child_id, &dwarf.strings);
                        if type_matches(die_ref, ty) {
                            type_found = true;
                        }
                    }
                }
                if !type_found {
                    let ty_id = unit.add(unit.root(), DW_TAG_base_type);
                    let die_ref = DIERef::new(unit, ty_id, &dwarf.strings);
                    create_type(die_ref);
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
                            let die_ref = DIERef::new(unit, die_id, &dwarf.strings);
                            update_fn(die_ref, fn_map);
                        }
                    }
                }
            }
            // Add a subprogram DIE for each remaining `fn_map` entry
            if let Some(fn_map) = &mut anvill_fn_map {
                let remaining_entries = fn_map.keys().cloned().collect::<Vec<_>>();
                for addr in remaining_entries {
                    let fn_id = unit.add(unit.root(), DW_TAG_subprogram);
                    let die_ref = DIERef::new(unit, fn_id, &dwarf.strings);
                    create_fn(die_ref, addr, &mut fn_map);
                }
            }
        }
        Ok(())
    })?;

    write_sections(&updated_sections)
}
