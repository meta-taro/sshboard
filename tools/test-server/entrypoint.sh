#!/bin/bash
set -e

# 公開鍵は起動時に渡す。**イメージへ焼かない。**
if [ -n "$AUTHORIZED_KEY" ]; then
  install -d -m 700 -o probe -g probe /home/probe/.ssh
  printf '%s\n' "$AUTHORIZED_KEY" > /home/probe/.ssh/authorized_keys
  chown probe:probe /home/probe/.ssh/authorized_keys
  chmod 600 /home/probe/.ssh/authorized_keys
fi

nohup /usr/local/bin/grow.sh >/dev/null 2>&1 &

exec /usr/sbin/sshd -D -e
