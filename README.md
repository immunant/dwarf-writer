# Example usage:

```
# $DISASM_DATA currently only supports anvill but is not actually used to change the binary yet
$ cargo run $DISASM_DATA $BINARY

$ ls debug_*
debug_abbrev debug_info debug_line debug_str

$ for section in debug_*; do objcopy --update-section .$section=$section $BINARY; done

$ objdump -g $BINARY | less

# To view a particular section
$ objdump -s -j $SECTION $BINARY
```
