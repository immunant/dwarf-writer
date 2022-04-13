use crate::anvill::AnvillInput;
use crate::dwarf_unit::DwarfUnitRef;
use crate::elf::ELF;
use crate::ghidra::GhidraInput;
use crate::str_bsi::StrBsiInput;
use crate::symbols::Symbols;
use anyhow::{Error, Result};
use clap::Parser;
use serde::Deserialize;
use simple_log::LogConfigBuilder;
use std::path::Path;
use std::path::PathBuf;
use std::{fs, io};

mod anvill;
mod dwarf_attr;
mod dwarf_entry;
mod dwarf_unit;
mod elf;
mod ghidra;
mod into_gimli;
mod str_bsi;
mod symbols;
mod types;

#[derive(Parser, Debug)]
#[clap(name = "dwarf-writer")]
pub struct Opt {
    #[clap(name = "input", help = "Input binary", parse(from_os_str))]
    input_binary_path: PathBuf,
    #[clap(name = "output", help = "Output binary", parse(from_os_str))]
    output_binary_path: Option<PathBuf>,
    #[clap(
        name = "anvill-data",
        short = 'a',
        long = "anvill",
        help = "Anvill disassembly data",
        parse(from_os_str)
    )]
    anvill_paths: Vec<PathBuf>,
    #[clap(
        name = "str-data",
        short = 'b',
        long = "str-bsi",
        help = "STR BSI disassembly data",
        parse(from_os_str)
    )]
    str_bsi_paths: Vec<PathBuf>,
    #[clap(
        name = "ghidra",
        short = 'g',
        long = "ghidra",
        help = "Ghidra disassembly data",
        parse(from_os_str)
    )]
    ghidra_paths: Vec<PathBuf>,
    #[clap(
        short = 'u',
        long = "use-all-str",
        help = "Use all entries in STR data regardless of confidence level"
    )]
    use_all_str: bool,
    #[clap(
        name = "output-dir",
        short = 's',
        long = "section-files",
        help = "Output directory for writing DWARF sections to individual files",
        parse(from_os_str)
    )]
    output_dir: Option<PathBuf>,
    #[clap(
        name = "objcopy-path",
        short = 'x',
        long = "objcopy",
        help = "Alternate objcopy to use (defaults to objcopy in PATH)",
        parse(from_os_str)
    )]
    objcopy_path: Option<PathBuf>,
    #[clap(
        name = "omit-variables",
        long = "omit-variables",
        help = "Avoid emitting DW_TAG_variable entries for Anvill"
    )]
    omit_variables: bool,
    #[clap(
        name = "omit-functions",
        long = "omit-functions",
        help = "Avoid emitting DW_TAG_subprogram entries"
    )]
    omit_functions: bool,
    #[clap(long = "omit-symbols", help = "Avoid adding ELF symbols")]
    omit_symbols: bool,
    #[clap(short = 'v', long = "verbose")]
    verbose: bool,
    // Has precedence over `verbose` flag
    #[clap(
        name = "level",
        short = 'l',
        long = "logging",
        help = "Set logging level explicitly",
        parse(from_str)
    )]
    logging: Option<String>,
}

pub trait InputFile: Sized + for<'de> Deserialize<'de> {
    /// Loads a file to create a new `AnvillInput`.
    fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let hints = serde_json::from_reader(reader)?;
        Ok(hints)
    }
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    let log_level = opt
        .logging
        .as_deref()
        .unwrap_or(if opt.verbose { "trace" } else { "info" });
    let log_config = LogConfigBuilder::builder()
        .level(log_level)
        .output_console()
        .build();
    simple_log::new(log_config).map_err(Error::msg)?;

    let mut elf = ELF::new(&opt.input_binary_path)?;

    let mut dwarf = DwarfUnitRef::new(&mut elf);

    let mut syms = Symbols::new();

    let mut type_map = dwarf.create_type_map();

    for path in &opt.ghidra_paths {
        let input = GhidraInput::new(path)?;
        let ghidra_data = input.data()?;
        if !opt.omit_symbols {
            syms.add_ghidra(&ghidra_data);
        }
        dwarf.process_ghidra(ghidra_data, &mut type_map);
    }

    for path in &opt.anvill_paths {
        let input = AnvillInput::new(path)?;
        let anvill_data = input.data(&opt);
        if !opt.omit_symbols {
            syms.add_anvill(&anvill_data);
        }
        dwarf.process_anvill(anvill_data, &mut type_map);
    }

    for path in &opt.str_bsi_paths {
        let input = StrBsiInput::new(path)?;
        dwarf.process_str_bsi(input.data(&opt), &mut type_map);
    }

    elf.update_binary(
        opt.output_binary_path,
        opt.objcopy_path,
        opt.output_dir,
        syms,
    )?;

    Ok(())
}
