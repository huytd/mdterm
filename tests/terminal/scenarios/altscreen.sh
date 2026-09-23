#!/bin/bash
printf '\033[H\033[2J'
printf 'Normal buffer: before alternate screen\n'
printf 'Normal line 2\n'

# Enter alternate screen buffer and save cursor
printf '\033[?1049h'
printf '\033[H\033[2J'
printf '=== Alternate Screen Buffer ===\n'
printf 'This is running inside 1049 alternate screen.\n'
printf 'It should not leak to normal screen upon exit.\n'

# Exit alternate screen buffer and restore cursor
printf '\033[?1049l'
printf 'Normal buffer: returned from alternate screen\n'
printf 'Final normal buffer content.\n'
sleep 2
