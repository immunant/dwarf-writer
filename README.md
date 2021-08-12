# Dwarf-writer

Dwarf-writer updates DWARF debug sections with information obtained through disassembly. The target program's debug info can be either updated in-place or written to a copy of the program. Writing the updated debug sections to individual files is also supported.

## Building and prerequisites

Building dwarf-writer requires a [rust installation](https://www.rust-lang.org/) and [objcopy](https://www.gnu.org/software/binutils/) cross-compiled for the target program's architecture.

```
$ git clone https://github.com/immunant/dwarf-writer

$ cd dwarf-writer

$ cargo build --release

# To run dwarf-writer
$ cargo run -- -a $ANVILL_JSON -x /path/to/objcopy $BINARY

# dwarf-writer can also be run without cargo
$ ./target/release/dwarf-writer -a $ANVILL_JSON -x /path/to/objcopy $BINARY

# To install dwarf-writer
$ cargo install --path .
```

## Supported target architectures

Target architecture support depends on the input disassembly data sources. Currently the only disassembly data source supported is a limited subset of the [JSON specification](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md) produced by [Anvill's](https://github.com/lifting-bits/anvill/) python plugin which supports x86, ARM and SPARC. See the [spec docs](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md#architecture) for details.

## Example usage:

```
$ dwarf-writer -h
dwarf-writer 0.1.0

USAGE:
    dwarf-writer [FLAGS] [OPTIONS] <input> [output]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    

OPTIONS:
    -a, --anvill <anvill-path>           Read anvill disassembly data
    -l, --logging <logging>              Set logging level explicitly
    -x, --objcopy_path <objcopy-path>    Specify path to objcopy
    -o, --output_dir <output-dir>        Set output directory for updated DWARF sections

ARGS:
    <input>     Input binary
    <output>    Output binary

# To update the program's debug info in-place
$ dwarf-writer -a $ANVILL_JSON $BINARY

# To update the debug info in a copy of the program
$ dwarf-writer -a $ANVILL_JSON $IN_BINARY $OUT_BINARY

# Specify an alternate objcopy path to run dwarf-writer on binaries for other architectures
$ dwarf-writer -a $ANVILL_JSON -x /usr/bin/arm-linux-gnueabihf-objcopy $BINARY

# To view the program's updated debug info
$ llvm-dwarfdump $BINARY
```
