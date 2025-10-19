#!/bin/bash
if [ "$1" == "--release" ];
then
    cargo build --release && \
    cp target/release/libremu_backend.so . -v && \
    echo 'Everething is ok. Running bot now.' && \
    python3 remu.py;
else
    cargo build && \
    cp target/debug/libremu_backend.so . -v && \
    echo 'Everething is ok. Running bot now.' && \
    RUST_BACKTRACE=1 python3 remu.py --verbose --one-poll;
fi
