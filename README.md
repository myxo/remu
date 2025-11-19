# Remu 
Remu - telegram bot witch remind you about events. Backend written in rust, frontend (http api talker) in python with telebot library.

> Note: right now I am in process of rewriting this project to 100% rust (and large refactoring), so I can launch it again.

## Debian package

You can build an installable `.deb` that ships the release binary and the
systemd service by running:

```
scripts/build-deb.sh
```

By default the script pulls the version from `Cargo.toml`. Set `PKG_VERSION`
when you need to override it (for example CI tag builds). The resulting package
lands in `target/debian/`.

The package installs a `remu` system user, drops the binary into `/usr/bin`,
and installs `remu.service` under `/lib/systemd/system`. Place your bot token
into `/var/lib/remu/token.id`, ensure it is owned by the `remu` user, then
enable the service:

```
sudo systemctl enable --now remu.service
```

Use `/etc/remu/remu.env` to pass extra environment, e.g. `RUST_LOG`.
