#!/bin/bash

set -e

cargo build --release

# set -x

readarray -t detected_gpus < <( ./target/release/dsim /dev/null -p list 2>/dev/null | grep 'max_compute_units' | sed 's/max_compute_units.*//g' | sed -e 's/[[:space:]]*$//' )

echo "=== Detected GPUs ==="
for gpu_name in "${detected_gpus[@]}" ; do
  echo " - $gpu_name"
done

for gpu_name in "${detected_gpus[@]}" ; do
  echo "Testing simulation on GPU '$gpu_name'"
  ./target/release/dsim example-data/simcontrol.toml -n 128 -p "$gpu_name" -vvv
done
