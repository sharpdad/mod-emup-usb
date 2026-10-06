# MOD-EMUP adapter on a breadboard

The same circuit as the PCB (`hardware/`), generated from its description by
`tools/breadboard.py`, for a prototype without ordering a board. Power comes
from an old PC (ATX) power supply: the MOD-EMUP was a PC card, so its
+5 / +12 / -12 V are exactly what an ATX supply gives. That replaces the
PCB's 12 V jack, diode and two DC-DC modules.

**Check the DB25 pinout with a meter against your own box and cable before
powering anything** (from Tom Trebisky's cable page; see `../README.md`).

## Parts

* Arduino Nano (5 V), USB to the computer (it powers only itself; grounds are shared)
* 74HCT245, 74HCT244 (DIP-20), two full-size breadboards
* DB25 **female** breakout board with screw terminals (to the box's cable)
* ATX power supply; to switch it on, tie the green wire (PS_ON) to a black one
* 4 x 10k, 1k, an LED, 220 uF 10 V and 2 x 100 nF on +5 V; jumper wire

## Power

| Rail | From | To |
|---|---|---|
| +5V | ATX red (+5 V) | 74HCT244 pin 20, 74HCT245 pin 20, C2 (220u 10V) +, C4 (100n), C5 (100n), DB25 pin 11, DB25 pin 15, DB25 pin 24, DB25 pin 3, R1 (10k), R2 (10k), R3 (10k), R4 (10k), R5 (1k) |
| +12V | ATX yellow (+12 V) | DB25 pin 16, DB25 pin 23 |
| -12V | ATX blue (-12 V) | DB25 pin 4 |
| GND | ATX black (GND) | 74HCT244 pin 1, 74HCT244 pin 10, 74HCT244 pin 11, 74HCT244 pin 13, 74HCT244 pin 15, 74HCT244 pin 17, 74HCT244 pin 19, 74HCT245 pin 10, 74HCT245 pin 19, C2 (220u 10V) -, C4 (100n), C5 (100n), D2 LED cathode (short leg), DB25 pin 1, DB25 pin 12, DB25 pin 13, DB25 pin 14, DB25 pin 2, DB25 pin 25, Nano GND, Nano GND2 |

The Nano's own +5 V pin stays unconnected (it runs from USB).

## Signals

Each line: everything on one net, wire them together.

| Net | Connect |
|---|---|
| BD0 | 74HCT245 pin 18, DB25 pin 5 |
| BD1 | 74HCT245 pin 17, DB25 pin 18 |
| BD2 | 74HCT245 pin 16, DB25 pin 6 |
| BD3 | 74HCT245 pin 15, DB25 pin 19 |
| BD4 | 74HCT245 pin 14, DB25 pin 7 |
| BD5 | 74HCT245 pin 13, DB25 pin 20 |
| BD6 | 74HCT245 pin 12, DB25 pin 8 |
| BD7 | 74HCT245 pin 11, DB25 pin 21 |
| DIR | Nano D10, 74HCT245 pin 1 |
| LED | R5 (1k), D2 LED anode |
| ND0 | Nano D2, 74HCT245 pin 2 |
| ND1 | Nano D3, 74HCT245 pin 3 |
| ND2 | Nano D4, 74HCT245 pin 4 |
| ND3 | Nano D5, 74HCT245 pin 5 |
| ND4 | Nano D6, 74HCT245 pin 6 |
| ND5 | Nano D7, 74HCT245 pin 7 |
| ND6 | Nano D8, 74HCT245 pin 8 |
| ND7 | Nano D9, 74HCT245 pin 9 |
| N_RDDAT | Nano A2, 74HCT244 pin 6, R3 (10k) |
| N_RESET | Nano A3, 74HCT244 pin 8, R4 (10k) |
| N_WRDAT | Nano A1, 74HCT244 pin 4, R2 (10k) |
| N_WRID | Nano A0, 74HCT244 pin 2, R1 (10k) |
| /RDDAT | 74HCT244 pin 14, DB25 pin 9 |
| /RESET | 74HCT244 pin 12, DB25 pin 17 |
| /WRDAT | 74HCT244 pin 16, DB25 pin 22 |
| /WRID | 74HCT244 pin 18, DB25 pin 10 |

The 74HCT244's unused half (pins 11, 13, 15, 17) and both enables (1, 19)
go to GND, as listed under GND above; the 74HCT245's enable (19) too.
