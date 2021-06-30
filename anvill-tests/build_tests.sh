mkdir -p bin
mkdir -p json
cd src/
for i in *; do gcc -nostdlib $i -o ../bin/$i.gcc.elf; done
for i in *; do gcc -g -nostdlib $i -o ../bin/$i.debug.gcc.elf; done
for i in *; do clang -nostdlib $i -o ../bin/$i.clang.elf; done
for i in *; do clang -g -nostdlib $i -o ../bin/$i.debug.clang.elf; done
cd ../bin/
for i in *; do strip $i -o stripped.$i; done
for i in *; do python3 -m anvill --bin_in $i --spec_out ../json/$i.json; done
