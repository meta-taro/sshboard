#!/bin/bash
# 増え続けるログ。`tail -f` を試すため（Issue 005）。
# **色付きの行も混ぜる。**GUI は色付き / MCP はプレーン、を確かめるため。
i=0
while true; do
  i=$((i + 1))
  printf '%s \033[32mINFO\033[0m  tick %d\r\n' "$(date '+%H:%M:%S')" "$i" >> /home/probe/app/logs/app.log
  if [ $((i % 5)) -eq 0 ]; then
    printf '%s \033[31mERROR\033[0m disk pressure\r\n' "$(date '+%H:%M:%S')" >> /home/probe/app/logs/app.log
  fi
  sleep 1
done
