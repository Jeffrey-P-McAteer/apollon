#!/bin/bash

set -e

cargo build --release

echo ''

set -x

./target/release/dsim example-data/entities.csv example-data/deltas.json -n 128 -p nvidia

./target/release/dsim example-data/entities.csv example-data/deltas.json -n 128 -p intel


