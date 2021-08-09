#!/bin/bash

./clean_dirs.sh

# Use -nostdlib to keep the binaries small for now
cflags="-nostdlib"

for test_case in src/*
do
    name=$(basename $test_case)
    binary=bin/$name.elf
    stripped=strip_bin/$name.elf
    spec=anvill_json/strip.$name.json
    debug=bin/debug.$name.elf
    clang $cflags $test_case -o $binary
    # Only used for manually debugging tests
    clang $cflags -g $test_case -o $debug
    strip $binary -o $stripped
    python3 -m anvill --bin_in $stripped --spec_out $spec
done
