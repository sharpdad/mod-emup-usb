# The USB adapter PCB

`emup_adapter.py` describes the board (parts, nets, placement);
`./build.sh` turns it into the KiCad schematic (`emup_adapter.kicad_sch`,
`.pdf`), a routed PCB (`emup_adapter.kicad_pcb`), DRC / ERC reports and fab
files (`fab/`: Gerber + drill zip, BOM). The KiCad files open and edit
normally in KiCad 8; if you edit the board by hand, don't re-run build.sh.

* Power: one 12 V DC supply (2.1 mm barrel, centre positive, 2 A) through
  a 1N5822. +12 V goes to the box as is; +5 V (2 A) comes from a RECOM
  R-78B5.0-2.0 (7805-footprint switching module); -12 V from a TRACO TMR
  1-1211 isolated module with its +output tied to ground (about 80 mA). The
  power nets are 1 mm tracks (`netclasses.json`).
* Logic: an Arduino Nano (5 V, on USB) drives D0-D7 through a 74HCT245
  (direction from D10) and the strobes /WRID, /WRDAT, /RDDAT and /RESET
  through a 74HCT244 (from A0-A3). 10k pull-ups hold the strobes idle while
  the Nano boots. The buffers run from the board's +5 V.
* DB25 female, wired as the original SAC-201 card (pinout in
  ../docs/NOTES.md). Check it against your box first.

Untested. Things to check on the first board: the box's current on each
rail, the -12 V module's load, the DB25 pinout, and the strobe timing (the
firmware uses ~1 us strobes, like the ISA bus).
