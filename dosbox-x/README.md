# DOSBox-X with an EMUP stand-in

Records every access the DOS programming software makes to the SAC-201
card's two ports, so the chip algorithms (voltages, pin set-ups, pulse
timings) can be read off without disassembling the programs.

Build (macOS):

    git clone --depth 1 https://github.com/joncampbell123/dosbox-x.git ~/src/dosbox-x
    cd ~/src/dosbox-x
    cp /path/to/this/repo/dosbox-x/emup.cpp src/hardware/
    git apply /path/to/this/repo/dosbox-x/hooks.patch
    PATH="/opt/homebrew/opt/libtool/libexec/gnubin:/opt/homebrew/bin:$PATH" ./build-macos-sdl2

Run with `EMUP_BASE=2E0 EMUP_LOG=emup.log src/dosbox-x ...`. Log lines:
`<ms> I <id>` (ID port write), `<ms> W <id> <val>`, `<ms> R <id> <val>`.
Tested with a small COM program (I E5 / W E5 7F / R E5 7F).

## What the logs show so far (2026-10-04)

* The DOS programs use base 2E0 by default (menu option 5 changes it).
* **Identification** (every program, at start and before each operation):
  a function `check(n, expect)` writes E7 = 00, 01, 00, 02, then n pulses
  (03, 02), then 00; selects register **F8** (not in all03info.txt) and
  compares its low nibble with `expect`. BPPGM calls it as (31,7), (23,2),
  (3,11), (22,9), (11,8). So the box has some clocked logic readable at F8;
  a plain binary counter, a 4040's bit pairs, small LFSRs and running
  totals don't fit, so it needs the real box (or its schematic) to settle.
  BPPGM.EXE's check is at file offset 6C82; patching the `jne` at 6D44 to
  two NOPs (scratch copy only) skips it.
* With the check skipped, BPPGM V3.16 (MOD-EMUP (C) 1992) reaches its menu
  (default chip MMI 63S080), and B (blank check) loops: the counter check,
  then ~1000 reads of E4 (socket pins 33-40), over and over: it waits for
  something the box reports there (chip / socket sense?).
* Next: the USB adapter, then a DOSBox-X build that forwards these port
  accesses to the real box over USB (and logs them): the box answers the
  checks itself and the logs give the real algorithms.

EMUP_SCREEN=file makes the device write the DOS text screen there 4x a
second; DOSBox-X's AUTOTYPE drives the menus (e.g. `autotype -w 8 -p 1.5 b`).
