#!/bin/bash

rm -f out_bin/*
./run_writer.sh
pytest
