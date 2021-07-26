use anvill::AnvillHints;
use anyhow::Result;
use dwarf_unit::process_anvill;
use elf::ELF;
use std::path::PathBuf;
use structopt::StructOpt;

mod anvill;
mod dwarf_attr;
mod dwarf_die;
mod dwarf_unit;
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

    let mut elf = ELF::new(&opt.binary_path)?;

    if let Some(path) = opt.anvill_path {
        let hints = AnvillHints::new(path)?;
        process_anvill(&mut elf, hints.ctxt());
    };

    let updated_sections = elf.sections()?;

    ELF::dump_sections(&updated_sections)?;

    Ok(())
}
