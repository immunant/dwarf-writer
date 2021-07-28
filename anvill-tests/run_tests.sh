cd bin/
mkdir -p tmp
for elf in *.elf
do
    echo "Running test" $elf
    cargo run -- -b $elf -a ../json/$elf.json -o tmp/
    RESULT=$?
    if [ $RESULT == 0 ]; then
        cd tmp
        for section in debug_*
        do
            objcopy --update-section .$section=$section ../$elf
            RESULT=$?
            if [ $RESULT == 0 ]; then
                echo "Updated section" .$section
            else
                objcopy --add-section .$section=$section ../$elf
                echo "Added section" .$section
            fi
        done
        cd ../
        rm tmp/debug_*
    else
        echo "Failed test" $elf
    fi
done
rmdir tmp
