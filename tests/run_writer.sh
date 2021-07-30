#!/bin/bash

for elf_path in bin/*.elf
do
    anvill_path=anvill_json/$(basename $elf_path).json
    echo "Running dwarf-writer on" $elf
    cargo run -- -b $elf_path -a $anvill_path
    result=$?
    if [ $result == 0 ]; then
        echo "Updated binary for test" $elf
    else
        echo "Failed test" $elf
    fi
done

for elf_path in strip_bin/*.elf
do
    anvill_path=anvill_json/$(basename $elf_path).strip.json
    echo "Running dwarf-writer on" $elf
    cargo run -- -b $elf_path -a $anvill_path
    result=$?
    if [ $result == 0 ]; then
        echo "Updated binary for test" $elf
    else
        echo "Failed test" $elf
    fi
done
