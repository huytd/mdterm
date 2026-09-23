#!/bin/bash
printf '\033[H\033[2J'
printf '=== Powerline Glyphs Test ===\n'
printf 'U+E0B0 (solid right):    \033[44;37m user@host \033[42;34m\xee\x82\xb0\033[42;30m /path/to/repo \033[49;32m\xee\x82\xb0\033[0m\n'
printf 'U+E0B1 (thin right):     \033[44;37m section A \033[37;44m\xee\x82\xb1\033[37;44m section B \033[0m\n'
printf 'U+E0B2 (solid left):     \033[49;35m\xee\x82\xb2\033[45;37m branch:main \033[45;36m\xee\x82\xb2\033[46;30m utf8 \033[0m\n'
printf 'U+E0B3 (thin left):      \033[46;30m col 1 \033[30;46m\xee\x82\xb3\033[30;46m ln 42 \033[0m\n'
printf 'All four together: \xee\x82\xb0 \xee\x82\xb1 \xee\x82\xb2 \xee\x82\xb3\n'
sleep 2
