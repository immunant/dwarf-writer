cd bin/
for elf in *.elf
do
    echo "Running test" $elf
    cargo run -- -b $elf -a ../json/$elf.json
    RESULT=$?
    if [ $RESULT == 0 ]; then
        echo "Updated binary for test" $elf
    else
        echo "Failed test" $elf
    fi
done
