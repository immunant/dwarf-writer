use anvill::AnvillInput;
use anyhow::Result;
use dwarf_unit::{create_type_map, process_anvill};
use elf::ELF;
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
#[structopt(name = "basic")]
struct Opt {
    #[structopt(
        short = "b",
        long = "bin_in",
        help = "Input binary",
        parse(from_os_str)
    )]
    binary_path: PathBuf,
    #[structopt(
        short = "a",
        long = "anvill",
        help = "Optional input disassembly produced by anvill",
        parse(from_os_str)
    )]
    anvill_path: Option<PathBuf>,
    //#[structopt(short = "m", long = "mindsight", parse(from_os_str))]
    //mindsight_path: Option<PathBuf>,
    #[structopt(
        short = "o",
        long = "output_dir",
        help = "Optional output directory to store updated DWARF sections in",
        parse(from_os_str)
    )]
    output_dir: Option<PathBuf>,
    #[structopt(
        short = "x",
        long = "objcopy_path",
        help = "Specify alternate path to objcopy",
        parse(from_os_str)
    )]
    objcopy_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut elf = ELF::new(&opt.binary_path)?;

    let mut type_map = create_type_map(&elf);

    if let Some(path) = opt.anvill_path {
        let input = AnvillInput::new(path)?;
        process_anvill(&mut elf, input.data(), &mut type_map);
    };

    elf.update_binary(opt.objcopy_path, opt.output_dir)?;

    Ok(())
}
