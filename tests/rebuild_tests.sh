#!/bin/bash

./clean_dirs.sh

cflags=-nostdlib
for test_case in src/*
do
    clang $cflags $test_case -o bin/$(basename $test_case).elf
    clang $cflags -g $test_case -o bin/$(basename $test_case).debug.elf
done

for test_bin in bin/*
do
    stripped_name=strip_bin/$(basename $test_bin)
    strip $test_bin -o $stripped_name
    python3 -m anvill --bin_in $test_bin --spec_out anvill_json/$(basename $test_bin).json
    python3 -m anvill --bin_in $stripped_name --spec_out anvill_json/$(basename $stripped_name).strip.json
done
