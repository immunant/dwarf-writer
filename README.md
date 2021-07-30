# Dwarf-writer

Dwarf-writer updates DWARF debug sections with information obtained through targeted disassembly.

## Building and prerequisites

Building dwarf-writer requires a [rust installation](https://www.rust-lang.org/) and [objcopy](https://www.gnu.org/software/binutils/) cross-compiled for the target architecture. While the binary's DWARF debug sections are updated in-place, dumping the sections to individual files is also supported.

```
$ git clone https://github.com/immunant/dwarf-writer

$ cd dwarf-writer

$ cargo build --release

$ ./target/release/dwarf-writer -b $BINARY -a $ANVILL_JSON
```

## Supported target architectures

Target architecture support depends on the input disassembly data sources. Currently the only disassembly data source supported is a limited subset of the [JSON specification](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md) produced by [Anvill's](https://github.com/lifting-bits/anvill/) python plugins which supports x86, ARM and SPARC. See the [spec docs](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md#architecture) for details.

## Example usage:

```
$ cargo run -- -h

USAGE:
    dwarf-writer [OPTIONS] --bin_in <binary-path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --anvill <anvill-path>           Optional input disassembly produced by anvill
    -b, --bin_in <binary-path>           Input binary
    -x, --objcopy_path <objcopy-path>    Specify alternate path to objcopy
    -o, --output_dir <output-dir>        Optional output directory to store updated DWARF sections in

$ cargo run -- -b $BINARY -a $ANVILL_JSON

# To run dwaf-writer on binaries for other architectures specify an alternate objcopy path
$ cargo run -- -b $ARM_BINARY -a $ANVILL_JSON -x /usr/bin/arm-none-eabi-objcopy

# To view the program's updated debug info
$ llvm-dwarfdump $BINARY | less
```

## Testing
```
$ cd tests/

$ ./rebuild_tests.sh

$ ./run_writer.sh

$ python3 test_utils/test_runner.py
```
