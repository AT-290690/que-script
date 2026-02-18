#!/bin/bash
./target/release/fez-rs --rs  && rustc ./example/dist/main.rs -C opt-level=3 -o ./example/dist/main.native && ./example/dist/main.native