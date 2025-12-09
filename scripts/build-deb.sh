#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARCH="${ARCH:-}"
if [[ -z "$ARCH" ]]; then
    if command -v dpkg >/dev/null 2>&1; then
        ARCH="$(dpkg --print-architecture)"
    else
        ARCH="$(uname -m)"
    fi
fi
case "$ARCH" in
    x86_64) ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) ;;
esac

if [[ -z "${PKG_VERSION:-}" ]]; then
    PKG_VERSION="$(cargo metadata --format-version 1 --no-deps |
        python3 -c 'import json,sys; data=json.load(sys.stdin); print(data["packages"][0]["version"])')"
else
    PKG_VERSION="$PKG_VERSION"
fi

BUILD_DIR="$ROOT_DIR/target/debian"
PKG_NAME="remu_${PKG_VERSION}_${ARCH}"
PKG_ROOT="$BUILD_DIR/$PKG_NAME"

mkdir -p "$BUILD_DIR"
rm -rf "$PKG_ROOT"

cargo build --release

mkdir -p \
    "$PKG_ROOT/DEBIAN" \
    "$PKG_ROOT/usr/bin" \
    "$PKG_ROOT/lib/systemd/system" \
    "$PKG_ROOT/etc/remu" \
    "$PKG_ROOT/var/lib/remu" \
    "$PKG_ROOT/usr/share/doc/remu"

install -m 0755 "$ROOT_DIR/target/release/remu" "$PKG_ROOT/usr/bin/remu"
install -m 0644 "$ROOT_DIR/packaging/systemd/remu.service" "$PKG_ROOT/lib/systemd/system/remu.service"
install -m 0755 "$ROOT_DIR/packaging/debian/postinst" "$PKG_ROOT/DEBIAN/postinst"
install -m 0755 "$ROOT_DIR/packaging/debian/prerm" "$PKG_ROOT/DEBIAN/prerm"
install -m 0644 "$ROOT_DIR/packaging/README.Debian" "$PKG_ROOT/usr/share/doc/remu/README.Debian"

cat > "$PKG_ROOT/DEBIAN/control" <<CONTROL
Package: remu
Version: $PKG_VERSION
Architecture: $ARCH
Section: utils
Priority: optional
Maintainer: Klimov Nikolay <klimov.ns@mail.ru>
Depends: adduser, libc6 (>= 2.31), libsqlite3-0, systemd
Description: Remu Telegram reminder bot
 The Remu bot processes Telegram updates and schedules reminders.
CONTROL

DPKG_CMD=(dpkg-deb --build)
if dpkg-deb --help 2>&1 | grep -q -- '--root-owner-group'; then
    DPKG_CMD+=(--root-owner-group)
fi
DPKG_CMD+=("$PKG_ROOT")

"${DPKG_CMD[@]}"

echo "Built package: $BUILD_DIR/$PKG_NAME.deb"
