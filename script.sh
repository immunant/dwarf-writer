if [ -n "$1" ]; then
    echo "Updating DWARF data in "$1" with debug_* in current directory"
    for section in debug_*; do
        objcopy --update-section .$section=$section $1;
    done
else
    echo "Provide a binary to update"
fi
