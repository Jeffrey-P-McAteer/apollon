
# Apollon (Discrete Event Simulator)

`apollon` is a small rust utility to perform GPU-accelerated discrete event simulations
given an initial state, state change functions, and some definition of a time domain over which to iterate.

# Example Sim Output

![Example](example.mov)

# Testing

```bash
cargo run --release -- example-data/entities.csv example-data/deltas.json -n 128

cargo build --release && ( ./target/release/apollon example-data/entities.csv example-data/deltas.json -n 128 -p nvidia ; ./target/release/apollon example-data/entities.csv example-data/deltas.json -n 128 -p intel )

# Linux platforms only
./test.sh

./test.sh -o /dev/stdout --num-steps 3000 --capture-step-period 100 --gis-color-attr color --output-animation-file-path /tmp/a.gif --data-constant red_entity_speed_coef=0.08 && mpv --loop /tmp/a.gif


./test.sh -o /dev/stdout --num-steps 9000 --capture-step-period 100 --output-animation-frame-delay 41 --gis-color-attr color --output-animation-file-path example.mp4 --data-constant red_entity_speed_coef=0.08 -v && mpv --loop example.mp4


```

# Name

From [theoi.com/Olympios](https://www.theoi.com/Olympios/Apollon.html) -

> APOLLON (Apollo) was the Olympian god of prophecy and oracles, music, song and poetry, archery, healing, plague and disease, and the protection of the young.
>
> ...[He slew] the serpent Python which guarded the oracular shrine of Delphoi (Delphi).

Discrete event simulators are often used as a type of prophecy or oracle allowing us to peer into the future given some initial state.
I also find the myth about slaying the serpent Python a great prophecy in itself as [Python](https://www.python.org/) is typically used for
ad-hoc event simulators which this tool aims to replace with high-performance OpenCL primitives and connecting simulaton config files.




