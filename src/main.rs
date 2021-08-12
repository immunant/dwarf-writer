use anvill::AnvillInput;
use anyhow::{Error, Result};
use dwarf_unit::DwarfUnitRef;
use elf::ELF;
use simple_log::LogConfigBuilder;
use std::path::PathBuf;
use structopt::StructOpt;

mod anvill;
mod dwarf_attr;
mod dwarf_entry;
mod dwarf_unit;
mod elf;
mod into_gimli;
mod types;

#[derive(StructOpt, Debug)]
#[structopt(name = "dwarf-writer")]
struct Opt {
    #[structopt(name = "input", help = "Input binary", parse(from_os_str))]
    input_binary_path: PathBuf,
    #[structopt(name = "output", help = "Output binary", parse(from_os_str))]
    output_binary_path: Option<PathBuf>,
    #[structopt(
        short = "a",
        long = "anvill",
        help = "Read anvill disassembly data",
        parse(from_os_str)
    )]
    anvill_path: Option<PathBuf>,
    #[structopt(
        short = "o",
        long = "output_dir",
        help = "Set output directory for updated DWARF sections",
        parse(from_os_str)
    )]
    output_dir: Option<PathBuf>,
    #[structopt(
        short = "x",
        long = "objcopy_path",
        help = "Specify path to objcopy",
        parse(from_os_str)
    )]
    objcopy_path: Option<PathBuf>,
    #[structopt(short = "v", long = "verbose")]
    verbose: bool,
    // Has precedence over `verbose` flag
    #[structopt(
        short = "l",
        long = "logging",
        help = "Set logging level explicitly",
        parse(from_str)
    )]
    logging: Option<String>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
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

    let mut type_map = dwarf.create_type_map();

    if let Some(path) = opt.anvill_path {
        let input = AnvillInput::new(path)?;
        dwarf.process_anvill(input.data(), &mut type_map);
    };

    elf.update_binary(opt.output_binary_path, opt.objcopy_path, opt.output_dir)?;

    Ok(())
}
