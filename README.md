# Dwarf-writer

Dwarf-writer updates DWARF debug sections with information obtained through targeted disassembly.

## Building and prerequisites

Building dwarf-writer requires a [rust installation](https://www.rust-lang.org/). Dwarf-writer also currently writes the update sections to individual files instead of modifying the input binary. While this is subject to change, it means that currently [objcopy](https://www.gnu.org/software/binutils/) cross-compiled for the target architecture is required to write the updated sections back to the program binary.

```
$ git clone https://github.com/immunant/dwarf-writer

$ cd dwarf-writer

$ cargo build --release

$ ./target/release/dwarf-writer $DISASM_DATA $BINARY
```

## Supported target architectures

Target architecture support depends on the limits of the input disassembly data sources. Currently the only disassembly data source supported is the [JSON specification](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md) produced by [Anvill's](https://github.com/lifting-bits/anvill/) python plugins which supports x86, ARM and SPARC. See the [spec docs](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md#architecture) for details.

## Example usage:

```
$ cargo run $DISASM_DATA $BINARY

$ ls debug_*
debug_abbrev debug_info debug_line debug_str

$ for section in debug_*; do objcopy --update-section .$section=$section $BINARY; done

# To view program's update debug info
$ objdump -g $BINARY | less

# To view a particular section
$ objdump -s -j $SECTION $BINARY
```
