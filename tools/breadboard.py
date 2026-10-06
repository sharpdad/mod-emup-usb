#!/usr/bin/env python3
"""Breadboard wiring list for the MOD-EMUP USB adapter, generated from the
PCB's own description (hardware/emup_adapter.py), so the two can't drift.
Power from a PC (ATX) supply instead of the board's 12 V input and DC-DC
modules. Writes docs/BREADBOARD.md."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "..", "hardware", "emup_adapter.py")
OUT = os.path.join(HERE, "..", "docs", "BREADBOARD.md")

parts = []
ns = {"Part": lambda ref, sym, value, fp, conn, *a, **k: parts.append((ref, value, conn))}
exec(open(SRC).read(), ns)

# on the breadboard: no barrel jack, diode, DC-DC modules or their bulk caps
SKIP = {"J1", "D1", "U1", "U2", "C1", "C3", "FLG2", "FLG5"}
NANO_NAME = {v: k for k, v in ns["NANO"].items()}
label = {"A1": "Nano", "U3": "74HCT245", "U4": "74HCT244", "J2": "DB25"}


def pin_name(ref, pin):
    p = int(pin.lstrip("#"))
    if ref == "A1":
        return "Nano %s" % NANO_NAME.get(p, "pin %d" % p)
    if ref == "J2":
        return "DB25 pin %d" % p
    if ref in label:
        return "%s pin %d" % (label[ref], p)
    value = dict((r, v) for r, v, _ in parts)[ref]
    if ref.startswith("R") or value == "100n":
        return "%s (%s)" % (ref, value)              # (either end)
    if ref.startswith("D"):                          # KiCad LED: 1 cathode, 2 anode
        return "%s LED %s" % (ref, "cathode (short leg)" if p == 1 else "anode")
    return "%s (%s) %s" % (ref, value, "+" if p == 1 else "-")


nets = {}
for ref, value, conn in parts:
    if ref in SKIP:
        continue
    for pin, net in conn.items():
        nets.setdefault(net, []).append(pin_name(ref, pin))
power = {"+5V": "ATX red (+5 V)", "+12V": "ATX yellow (+12 V)", "-12V": "ATX blue (-12 V)", "GND": "ATX black (GND)"}
L = ["# MOD-EMUP adapter on a breadboard", "",
     "The same circuit as the PCB (`hardware/`), generated from its description by",
     "`tools/breadboard.py`, for a prototype without ordering a board. Power comes",
     "from an old PC (ATX) power supply: the MOD-EMUP was a PC card, so its",
     "+5 / +12 / -12 V are exactly what an ATX supply gives. That replaces the",
     "PCB's 12 V jack, diode and two DC-DC modules.", "",
     "**Check the DB25 pinout with a meter against your own box and cable before",
     "powering anything** (from Tom Trebisky's cable page; see `../README.md`).", "",
     "## Parts", "",
     "* Arduino Nano (5 V), USB to the computer (it powers only itself; grounds are shared)",
     "* 74HCT245, 74HCT244 (DIP-20), two full-size breadboards",
     "* DB25 **female** breakout board with screw terminals (to the box's cable)",
     "* ATX power supply; to switch it on, tie the green wire (PS_ON) to a black one",
     "* 4 x 10k, 1k, an LED, 220 uF 10 V and 2 x 100 nF on +5 V; jumper wire", "",
     "## Power", "",
     "| Rail | From | To |", "|---|---|---|"]
for net in ("+5V", "+12V", "-12V", "GND"):
    L.append("| %s | %s | %s |" % (net, power[net], ", ".join(sorted(set(nets.get(net, []))))))
L += ["", "The Nano's own +5 V pin stays unconnected (it runs from USB).", "",
      "## Signals", "", "Each line: everything on one net, wire them together.", "",
      "| Net | Connect |", "|---|---|"]
for net in sorted(nets):
    if net in power:
        continue
    L.append("| %s | %s |" % (net.replace("~", "/"), ", ".join(nets[net])))
L += ["", "The 74HCT244's unused half (pins 11, 13, 15, 17) and both enables (1, 19)",
      "go to GND, as listed under GND above; the 74HCT245's enable (19) too.", ""]
open(OUT, "w").write("\n".join(L))
print("wrote", OUT, len(nets), "nets")
