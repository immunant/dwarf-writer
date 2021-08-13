# Disassembly data sources

## Anvill's python plugin

Currently the only disassembly data source supported is a subset of the [JSON specification](https://github.com/lifting-bits/anvill/blob/master/docs/SpecificationFormat.md) produced by [Anvill's](https://github.com/lifting-bits/anvill/) python plugin which supports x86, ARM and SPARC. See the spec docs for details on what it can provide.

### Setting it up

Only the python plugin in anvill is required to produce disassembly data. This plugin requires either [Binary Ninja](https://docs.binary.ninja/getting-started.html) or [IDA PRO](https://hex-rays.com/ida-pro/) which are used as backends. After installing one of these disassemblers install the necessary parts of anvill with the following steps.

```
$ git clone https://github.com/lifting-bits/anvill

$ python3 -m pip install anvill/
```

### Basic usage
```
$ python3 -m anvill --bin_in $INPUT_BINARY --spec_out $OUTPUT_JSON
```
