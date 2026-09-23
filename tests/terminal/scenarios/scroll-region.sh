#!/bin/bash
printf '\033[H\033[2J'
# Fill lines 1-12
for i in $(seq 1 12); do
  printf 'Line %02d: initial content\n' "$i"
done

# Set scroll region from line 3 to line 8
printf '\033[3;8r'

# Move cursor inside scroll region and scroll by printing newlines
printf '\033[8;1H\n\n'
printf '\033[8;1H[Scrolled Line 1]'

# Origin mode ON (DECOM)
printf '\033[?6h'
# In origin mode (1,1) refers to top of margin (line 3)
printf '\033[1;1H[Origin Mode Line 1]'
# Insert line at line 1 of margin (line 3 of screen)
printf '\033[1L'
printf '\033[1;1H[Inserted Line]'
# Delete line 2 of margin
printf '\033[2;1H\033[1M'

# Test insert / delete characters
printf '\033[3;1HABCDEFGH'
printf '\033[3;4H\033[2@'  # Insert 2 spaces at pos 4
printf '\033[3;1H'
printf '\033[3;2H\033[1P'  # Delete 1 char at pos 2

# Reset origin mode and scroll region to full screen
printf '\033[?6l'
printf '\033[r'
printf '\033[12;1H=== Scroll Region Complete ===\n'
sleep 2
