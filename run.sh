#!/bin/bash
if [ "$1" == "--release" ];
then
    cargo build --release && \
    cp target/release/libtelegram_rust_backend.so . -v && \
    echo 'Everething is ok. Running bot now.' && \
    python3 remu.py;
else
    cargo build && \
    cp target/debug/libtelegram_rust_backend.so . -v && \
    echo 'Everething is ok. Running bot now.' && \
    python3 remu.py --verbose;
fi
