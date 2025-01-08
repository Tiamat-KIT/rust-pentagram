#!/bin/bash

# ログファイルのパス（適宜変更してください）
LOG_FILE="app.log"

# Render time の値を抜き出し、平均値を計算
average=$(grep -oP 'Render time:\s*[0-9]+' "$LOG_FILE" | awk '{sum += $1; count++} END {if (count > 0) print sum / count; else print 0}')

# 現在の時刻を取得
time_now=$(date '+%Y-%m-%d %H:%M:%S')

# 出力
printf "%s %.2fμs\n" "$time_now" "$average"
