#!/bin/bash
printf '\033[H\033[2J'
printf '┌─────────────────────── Tmux Resize Test ───────────────────────┐\n'
printf '│ This session starts at 80x24 and will be resized to 100x30.    │\n'
printf '│ Validating SIGWINCH handling and status line relocation.       │\n'
printf '└────────────────────────────────────────────────────────────────┘\n'
sleep 2
