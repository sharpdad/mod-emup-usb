# KiCad generator

Makes a KiCad 8 schematic and routed PCB from a Python board description
(see ../../hardware/emup_adapter.py): `gen_sch.py` (schematic: library
symbols with a net label on every pin), `gen_pcb.py` (footprints placed,
nets, outline, ground pours; run with KiCad's own Python, which has
pcbnew), `route.py` (autoroutes with Freerouting, then re-adds the pours).

Freerouting: download the jar from https://github.com/freerouting/freerouting
/releases into `tools/freerouting/` (needs Java 17+, e.g. `brew install
openjdk`). `route.py` looks for `tools/freerouting/freerouting-*.jar`.
