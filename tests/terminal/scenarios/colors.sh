#!/bin/bash
printf '\033[H\033[2J'
printf '=== SGR Style Attributes ===\n'
printf 'Normal:       \033[0mNormal text\033[0m\n'
printf 'Bold:         \033[1mBold text\033[0m\n'
printf 'Dim:          \033[2mDim text\033[0m\n'
printf 'Italic:       \033[3mItalic text\033[0m\n'
printf 'Underline:    \033[4mUnderlined text\033[0m\n'
printf 'Inverse:      \033[7mInverse text\033[0m\n'
printf 'Strikethrough:\033[9mStrikethrough text\033[0m\n'
printf 'Combined:     \033[1;3;4;9mBold+Italic+Underline+Strike\033[0m\n'

printf '\n=== 256 Color Palette Sample ===\n'
for i in 16 34 52 70 88 124 160 196 208 220 226; do
  printf '\033[48;5;%dm  \033[0m' "$i"
done
printf '\n'
for i in 17 35 53 71 89 125 161 197 209 221 227; do
  printf '\033[38;5;%dm%03d \033[0m' "$i" "$i"
done
printf '\n'

printf '\n=== 24-bit Truecolor Gradients ===\n'
for r in 0 64 128 192 255; do
  for g in 0 64 128 192 255; do
    printf '\033[48;2;%d;%d;120m \033[0m' "$r" "$g"
  done
done
printf '\n'
sleep 2
