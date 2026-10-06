# EMUP Studio

A modern app for the MOD-EMUP / ALL-03 universal programmer (see
`../README.md`). Tauri 2: the window is plain HTML/CSS/JS (`ui/`), the
Rust side (`src-tauri/`) does files, the serial port to the USB adapter and
the programming operations. Builds for macOS, Windows and Linux from the
same code.

Until the USB adapter exists, Read / Blank / Program / Verify run against a
simulated chip (blank at first; programming only clears bits, like real
fuses and EPROM cells), so the whole interface can be tried now.

## What it does

* Chip list with search (`src-tauri/devices.json`: bipolar PROMs incl.
  the Tempest Math Box parts, EPROMs, PALs, GALs, PEEL), each with its
  organisation, pins and the DOS program that handles it.
* A drawing of the 40-pin socket showing where the chip goes (pin 1,
  notch, bottom-justified as in the ALL-03 manual; to confirm on the box).
* Buffer: open or drag in .bin, Intel .hex or JEDEC .jed; hex view; sum16
  (Atari label style) and CRC32.
* Read, Blank check, Program, Verify with progress and a log.
* The programmer menu lists serial ports (the adapter will be one).

## Build and run

    cd src-tauri
    cargo tauri dev            # run with live reload of ui/
    cargo tauri build          # release app (macOS: .app; Windows: .exe/.msi)

Needs Rust (rustup) and `cargo install tauri-cli --version "^2"`. On
Windows also the WebView2 runtime (built into Windows 10/11).

## The box layer (`src-tauri/src/emup.rs`)

* `Bus`: write a register, queue a read, wait, flush. Two of them:
  * `SimBox`: a model of the box (registers per `../ref/all03info.txt`)
    with a virtual EPROM in the socket. It answers only when powered (VCC
    5 V on its VCC pin, pin 20 grounded) and selected (/CE, /OE low); open
    collector TTL; a byte programs only with VPP at the right voltage, /OE
    high and a 45-55 ms PGM pulse, and bits only go 1 -> 0.
  * `SerialBus`: the USB adapter (`../firmware/emup_usb/`), batched.
* Algorithms: `read`, `program` (with verify); blank check and verify are
  reads. 2716 and 2732 so far (2732 read only). Chips sit at the bottom of
  the socket: chip pin k at socket pin k + 20 - pins/2.
* `cargo test`: reads and programs a virtual 2716 through the model.

These follow the ALL-03 register document; the real box is the final word
(voltages, pin placement, timing). Other chip families still use the
simple stand-in in `main.rs`.

## Next

More algorithms: 2732/27xx programming, bipolar PROMs (BPPGM), PALs and
GALs (fuse maps from JEDEC), learned from the DOS software under DOSBox-X
(`../dosbox-x/`) and the register document.
