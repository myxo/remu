# Remu 
Remu - telegram bot witch remind you about events. Backend written in rust, frontend (http api talker) in python with telebot library.

## Run
Create file token.id, put there your bot token_id and run
```
$ touch token.id
$ echo <token_id> > token.id
$ run.sh --release
```

Tested in Ubuntu 16.04

Dependencies: rust 1.22, python3, sqlite3-dev, [telebot](https://github.com/eternnoir/pyTelegramBotAPI)

Optional (for voice recognition): opusdec, [asrclient-cli.py](https://github.com/yandex/speechkitcloud/tree/master/python)

For running as service make /etc/systemd/system/remu-bot.service:
```
[Unit]
Description=remu bot
After=network-online.target

[Service]
User=<your username>
Environment=PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:<path to cargo>
WorkingDirectory=<path to remu>
ExecStart=<path to remu>/run.sh --release

[Install]
WantedBy=multi-user.target
```

And ```systemctl start remu-bot```

PS. This bot was created mainly to learn rust. Don't expect much from it. 