
# Discrete Event Simulator

`dsim` is a small rust utility to perform GPU-accelerated discrete event simulations
given an initial state, state change functions, and some definition of a time domain over which to iterate.


# Testing

```bash
cargo run --release -- example-data/entities.csv example-data/deltas.json -n 128

cargo build --release && ( ./target/release/dsim example-data/entities.csv example-data/deltas.json -n 128 -p nvidia ; ./target/release/dsim example-data/entities.csv example-data/deltas.json -n 128 -p intel )

# Linux platforms only
./test.sh

```
