#!/usr/bin/env python3
import curses
import time

def main(stdscr):
    curses.curs_set(0)
    stdscr.clear()
    h, w = stdscr.getmaxyx()
    win_h, win_w = 10, 50
    win_y = max(0, (h - win_h) // 2)
    win_x = max(0, (w - win_w) // 2)
    
    win = curses.newwin(win_h, win_w, win_y, win_x)
    win.box()
    title = "mdterm Curses Test"
    win.addstr(1, (win_w - len(title)) // 2, title)
    win.addstr(3, 4, "Bordered curses window")
    win.addstr(5, 4, "Rendering accuracy test: PASS")
    win.addstr(7, 4, "Waiting for recorder...")
    
    win.refresh()
    stdscr.refresh()
    time.sleep(2)

if __name__ == '__main__':
    curses.wrapper(main)
