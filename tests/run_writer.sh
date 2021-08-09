#!/bin/bash

for test_case in src/*
do
    name=$(basename $test_case)
    stripped=strip_bin/$name.elf
    spec=anvill_json/strip.$name.json
    output=out_bin/strip.$name.elf
    echo "Running dwarf-writer on" $stripped
    cargo run -- $stripped $output -a $spec
    result=$?
    if [ $result == 0 ]; then
        echo "Updated binary for test" $name
    else
        echo "Failed test" $name
    fi
done
