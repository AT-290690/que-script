#!/bin/bash
./target/release/fez-rs --ml  && ocamlopt -c ./example/dist/main.ml && ocamlopt -o ./example/dist/main.native ./example/dist/main.cmx && ./example/dist/main.native