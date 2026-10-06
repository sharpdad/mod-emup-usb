# MOD-EMUP over USB

Goal: run the user's MOD-EMUP universal programmer (JDR Micro; the same
machine as the Hi-Lo ALL-03 / Tribal TUP-400 / Expro-60) from a modern
computer, replacing its 8-bit ISA card. It burns what modern programmers
won't: bipolar PROMs, old PALs, GALs, PEELs, MCUs.

## Folders

| | |
|---|---|
| `dos/` | The original DOS software (from Tom's `emup.tar`): one program per chip family, plus `ACCESS.EXE` (the menu). Not in git. |
| `samples/` | Tom's ROM / JEDEC images that came in the tar. Not in git. |
| `ref/` | Downloaded references: `all03info.txt` (register map), ALL-03A and Expro-60 schematics, the ALL-03 manual, the SAC-201B schematic, Expro PAL equations, Tom's page notes. Not in git. |
| `docs/` | Our notes. |

Sources: Tom Trebisky's pages, http://cholla.mmto.org/proms/emup/ (cable,
protocol, ISA card, Expro); ALL-03 software and docs via Matthieu Benoit's
pages and the xmission arcade archive.

## The ISA card (user's: PCB-SAC-201CE, P/N 04-1586)

4x 74LS138 + 74LS02 (address decode), 74LS245 (data), 74LS244 (strobes out),
100 pF filter caps on the cable lines, DB25 female. JP1 selects the base
address, 200-2F0 in 16-byte steps (the user's is on 200; the ALL-03 default
is 2E0). No intelligence on the card: it only extends four I/O strobes and
the data bus to the box. The box is all TTL, no microcontroller, and is
powered entirely from the ISA bus.

## The cable (DB25, straight through; male at the card, female at the box)

| Pin | Signal | Pin | Signal |
|---|---|---|---|
| 1 | GND | 14 | GND |
| 2 | GND | 15 | +5 V |
| 3 | +5 V | 16 | +12 V |
| 4 | -12 V | 17 | /RESET |
| 5 | D0 | 18 | D1 |
| 6 | D2 | 19 | D3 |
| 7 | D4 | 20 | D5 |
| 8 | D6 | 21 | D7 |
| 9 | /RD data port (base+2) | 22 | /WR data port (base+2) |
| 10 | /WR ID port (base+0) | 23 | +12 V |
| 11 | +5 V | 24 | +5 V |
| 12 | GND | 25 | GND |
| 13 | GND | | |

All strobes active low. (From Tom's cable page; check against our card with
a meter before powering anything.)

## The protocol

Two ports. Write a register ID to the **ID port**, then read or write that
register through the **data port**. Registers (ALL-03 `all03info.txt`):

| ID | Register |
|---|---|
| E0-E4 | TTL I/O, socket pins 1-40 (E0 bit 0 = pin 1 ... E4 bit 7 = pin 40). Write 1 before reading a pin. |
| E5 | VOP DAC: 0.1 V/step, 10.2-25.5 V (programming voltage) |
| E6 | VHH DAC: 0.06 V/step, 5.1-15.3 V |
| E7 | VCC DAC: 0.04 V/step, 0-10.2 V |
| E8-EC | VOP enable per pin (pins 1,5,7,9-32 only), via 22 ohm |
| ED-EE | VCC enable: ED b0-7 = pins 40,36,34,32,31,30,29,28; EE b0-5 = pins 27,26,9,7,5,1 |
| EF | "Other": ground select (pin 20 / 11 / 30 ...), VOP-2.4 V to pins 7, 25, clocks to pins 2/3 and 18/19 (Expro schematic: bits C B i2 i3 - i4 p25 p7 A) |
| F0-F4 | VHH enable per pin (pins 9-32) |
| F5-F6 | VHH direct (no resistor) to pins 28-32 |
| F7 | bit 3: GOOD LED |

Rules from the doc: every register is 0 after reset; set a pin's TTL bit
high before driving VOP/VHH/VCC on it or reading it (a low TTL output would
short the supply); the supplies drop ~0.6 V to the socket (VCC +15 steps,
VHH +10, VOP +6).

## Plan

1. **Adapter** (USB to DB25):
   * A 5 V microcontroller (e.g. ATmega32U4 Pro Micro, or an RP2040 with
     74HCT245/74HCT541 level buffers): 8 data lines, 3 strobes, /RESET.
   * **Power for the box**: +5 V at 2 A or so (the box is a lot of TTL;
     measure it), +12 V and -12 V at modest current. A small external
     supply (or a 12 V brick with a 5 V buck and a -12 V charge-pump/
     inverting module) feeding DB25 power pins. Not from USB.
   * Firmware runs **command lists**: write ID, write data, read data,
     delay N us, repeat; so programming pulses are timed on the adapter, not
     across USB (a USB round trip is ~1 ms; pulses are often 10 us-10 ms).
2. **Learn the algorithms** from the DOS software: run each program in a
   DOS emulator (DOSBox-X) with ports base..base+2 trapped and logged, and a
   simulated socket, to record the exact register sequences, voltages and
   timings per chip type (no disassembly needed). The ALL-03 open-source
   code is a second reference.
3. **Host tool** (Python, Mac): chip database + algorithms (start with what
   the user needs: bipolar PROMs (BPPGM), PAL16/20 (PALP1), GAL16V8/20V8/22V10
   (GAL1), 27xx EPROMs (EPP512)), read / blank check / program / verify,
   Intel hex / binary / JEDEC files.
4. Later, optionally: a DOSBox-X build that forwards the I/O to the adapter,
   to run the original software unchanged for rare chips.

## DOS programs (from the readmes)

| Program | Devices |
|---|---|
| EPP512 | 2716-27512, 2516-2564, 8764-87256, 2816-2864A, CY7C251/261/271 |
| EPP1024B / W | 1 Mbit+ byte-wide (27010, 271000, 27C1024 word: 1024W), TC572000/574000 |
| EEP1 | EEPROMs |
| SEEP1 | serial EEPROM NS 9306/07/46, 93Cxx |
| BPPGM | bipolar PROMs (non-registered) |
| PALP1-5 | PALs: 16R8/20R8 series, 16RA8/20RA10, 20G10/22V10, TI 22V10, AMD/Atmel 22V10 |
| GAL1-3 | GAL16V8/20V8/22V10, PALCE16V8/20V8/22V10, GAL6001 |
| PEEL1-2 | PEEL 18CV8, 22CV10, 20CG10 |
| FPLP1-2 | Signetics PLS153/173, PLHS18P8, PLC42VA12 |
| EPLD1 | Altera EP310/320/600/900 |
| PGM48 / PGM51 / PGMZ8 / MPU1 | 8748, 8751, Z8, 63701V0 / 7742 / 77C82 |
| ICTEST, PALTEST | logic IC test (TTL74.LIB, CMOS40/45.LIB), PLD vector test |
| ACCESS | menu that runs the others |

## Software (2026-10-04)

* `app/`: EMUP Studio, the Mac / Windows / Linux app (Tauri). Reads and
  programs 2716s (2732 read) through a model of the box today; the same
  code drives the real box through the adapter.
* `firmware/emup_usb/`: Arduino sketch for the USB adapter (data bus on
  D2-D9 through a 74LS245, strobes and /RESET on A0-A3 through a 74LS244;
  batched register ops over USB serial at 1 Mbaud).
