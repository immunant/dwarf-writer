rm bin/* json/*
mkdir -p bin
mkdir -p json
cd src/
for test_case in *; do gcc -nostdlib $test_case -o ../bin/$test_case.gcc.elf; done
for test_case in *; do gcc -g -nostdlib $test_case -o ../bin/$test_case.debug.gcc.elf; done
for test_case in *; do clang -nostdlib $test_case -o ../bin/$test_case.clang.elf; done
for test_case in *; do clang -g -nostdlib $test_case -o ../bin/$test_case.debug.clang.elf; done
cd ../bin/
for test_case in *; do strip $test_case -o stripped.$test_case; done
for test_case in *; do python3 -m anvill --bin_in $test_case --spec_out ../json/$test_case.json; done
