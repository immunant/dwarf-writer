#!/bin/bash

for elf_path in bin/*.elf
do
    binary=$(basename $elf_path)
    anvill_path=anvill_json/$binary.json
    output_path=out_bin/$binary
    echo "Running dwarf-writer on" $elf
    cargo run -- $elf_path $output_path -a $anvill_path
    result=$?
    if [ $result == 0 ]; then
        echo "Updated binary for test" $elf
    else
        echo "Failed test" $elf
    fi
done

for elf_path in strip_bin/*.elf
do
    binary=$(basename $elf_path)
    anvill_path=anvill_json/$(basename $elf_path).strip.json
    output_path=out_bin/$binary
    echo "Running dwarf-writer on" $elf
    cargo run -- $elf_path $output_path -a $anvill_path
    result=$?
    if [ $result == 0 ]; then
        echo "Updated binary for test" $elf
    else
        echo "Failed test" $elf
    fi
done
