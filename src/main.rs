use crate::anvill_parser::X86;
use anvill_parser::AnvillHints;
use anyhow::Result;
use dwarf_writer::ELF;
use gimli::constants::{DW_AT_name, DW_TAG_subprogram};
use gimli::write;
use std::env::{args, Args};
use std::io::Write;
use std::{fmt, fs, io};

mod anvill_parser;
mod dwarf_writer;

/// Defines information that may be provided by the input disassembly data.
pub trait Hints: fmt::Debug {
    /// Returns the names of symbols known to be functions.
    fn fn_names(&self) -> Option<Vec<&String>>;
}

fn print_help() {
    println!("help message goes here");
}

fn parse_args(args: &mut Args) -> Result<(String, String)> {
    let hints = args
        .skip(1)
        .next()
        .ok_or(anyhow::Error::msg("Missing hints file"))?;
    let binary = args
        .next()
        .ok_or(anyhow::Error::msg("Missing binary file"))?;
    Ok((hints, binary))
}

fn open_hints(path: &str) -> Result<AnvillHints<X86>> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let hints: AnvillHints<X86> = serde_json::from_reader(reader)?;
    Ok(hints)
}

fn main() -> Result<()> {
    let (hints_path, binary_path) = match parse_args(&mut args()) {
        Ok(paths) => paths,
        Err(e) => {
            print_help();
            return Err(e)
        },
    };
    let hints = open_hints(&hints_path)?;

    let fn_names = hints.fn_names();
    println!("{:?}", fn_names);

    let mut elf = ELF::new(&binary_path)?;
    // Note gimli has different types for accessing DWARF sections as read-only and
    // write. This means that the type of the arg in this closure differs from the
    // one below.

    // Traverses the DWARF data and prints out some stuff. `ELF::with_dwarf` returns
    // the result of the closure.
    elf.with_dwarf(|dwarf| {
        // For each unit
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;
            // For each DIE
            let mut entries = unit.entries();
            // Do a depth-first search traversal of the DIEs
            while let Some((delta_depth, entry)) = entries.next_dfs()? {
                // Consider only function DIEs
                if entry.tag() == DW_TAG_subprogram {
                    // Try getting the program name
                    let fn_name_attr = entry
                        .attr_value(DW_AT_name)?
                        .expect("does DW_TAG_subprogram always have DW_AT_name?");
                    let fn_name = dwarf.attr_string(&unit, fn_name_attr)?.to_string_lossy();
                    println!("fn name is {:?}", fn_name);

                    // For each attribute
                    let mut attrs = entry.attrs();
                    while let Some(attr) = attrs.next()? {
                        println!(
                            "{:?} > {:?} {:?} {:?}",
                            delta_depth,
                            attr.name().static_string(),
                            attr.value(),
                            attr.string_value(&dwarf.debug_str).map(|s| s.to_string())
                        );
                    }
                }
            }
        }
        Ok(())
    })?;

    // Traverses the DWARF data and changes functions named `f1` to `e1`.
    // `ELF::with_dwarf_mut` ignores the result of the closure and returns the
    // updated sections
    let updated_sections = elf.with_dwarf_mut(|dwarf| {
        // For each unit
        let num_units = dwarf.units.count();
        for idx in 0..num_units {
            let unit_id = dwarf.units.id(idx);
            let unit = dwarf.units.get_mut(unit_id);

            // Get the root DIE
            let root_die = unit.get(unit.root());

            // For each child DIE
            let children: Vec<_> = root_die.children().cloned().collect();
            for unit_entry_id in children {
                let die = unit.get_mut(unit_entry_id);
                // Consider only function DIEs
                if die.tag() == DW_TAG_subprogram {
                    for attr in die.attrs_mut() {
                        let fn_name = match attr.get() {
                            write::AttributeValue::String(s) => Some(s.clone()),
                            write::AttributeValue::StringRef(str_id) => {
                                Some(dwarf.strings.get(*str_id).to_vec())
                            },
                            _ => None,
                        };
                        fn_name.as_ref().map(|fn_name| {
                            let fn_name = std::str::from_utf8(fn_name).unwrap();
                            if fn_name == "f1" {
                                attr.set(write::AttributeValue::String("e1".as_bytes().to_vec()));
                                println!("changed function name from {:?} to \"e1\"", fn_name);
                            }
                        });
                    }
                }
            }
        }
        Ok(())
    })?;

    // Print modified sections to a file for use with objcopy
    updated_sections.for_each(|id, data| {
        if !data.slice().is_empty() {
            println!("Updating section {}", id.name());
            // Removes the leading '.' in the section name
            let file_name = &id.name()[1..];
            let mut file = std::fs::File::create(file_name)?;
            file.write(data.slice())?;
        }
        Ok(())
    })
}
