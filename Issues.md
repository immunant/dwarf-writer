# Compatibility Issues

## Ghidra

- Currently does not support DWARF version 5. For binaries without existing debug info dwarf-writer creates DWARF version 4 sections. Binaries with existing version 5 DWARF info may need to have it stripped then rewritten to have it work with ghidra.

