# reactive-synth-clock-divider
WASM implementation of a clock divider audio processing node compatible with the web audio API. Created for [reactive-synth](https://github.com/PatrickStephansen/reactive-synth), but usable elsewhere if I ever document how.

The clock divider is an AudioWorkletProcessor that generates a gate based on the incoming clock signal. When the signal rises above 0 it counts as a tick. The gate opens (outputs 1s) after a parameterized number of ticks. Similarly, when the signal drops to 0 or below, it counts as a tock and closes the gate (outputs 0s) once the tocks get to a separately parameterized number. Another input signal triggers the counters to reset to values that are also parameterized. The `attackAfterTicks`, `releaseAfterTocks`, `ticksOnReset`, and `tocksOnReset` parameters can be set to real numbers for more interesting patterns. The counters are reduced by the `attackAfterTicks` or `releaseAfterTocks` amounts when the output gate changes, not zeroed out, so remainders can make the gate change earlier the next time.

Alternatively to its main use for timing, it can be triggered at audio rates to create pulse waves including more interesting irregular ones. Changing the `attackAfterTicks` and `releaseAfterTocks` parameters can then produce an undertone series.

## build

build command:

```bash
cargo build --features wee_alloc --release --target=wasm32-unknown-unknown && \
wasm-opt -Oz --strip-debug -o worklet/reactive_synth_clock_divider.wasm \
target/wasm32-unknown-unknown/release/reactive_synth_clock_divider.wasm
```
Inspect size with:

```bash
twiggy top -n 20 target/wasm32-unknown-unknown/release/reactive_synth_clock_divider_opt.wasm
```

Run `npm link` from the worklet directory before trying to build the reactive-synth app (the dependent app not in this repo)

## test

`cargo test`
