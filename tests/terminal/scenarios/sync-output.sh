#!/bin/bash
printf '\033[H\033[2J'
# Begin synchronized update frame (DECSET 2026)
printf '\033[?2026h'
printf '┌──────────────────────── Synchronized Output Frame ────────────────────────┐\n'
printf '│ Frame rendered atomically with DECSET 2026                                │\n'
printf '│ Line 1: Atomic update test                                                 │\n'
printf '│ Line 2: No tearing between draw operations                                │\n'
printf '└───────────────────────────────────────────────────────────────────────────┘\n'
# End synchronized update frame (DECRST 2026)
printf '\033[?2026l'
sleep 2
