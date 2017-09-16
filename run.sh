cargo build && \
cp target/debug/libtelegram_rust_backend.so . -v && \
echo 'Everething is ok. Running bot now.' && \
python3 remu.py