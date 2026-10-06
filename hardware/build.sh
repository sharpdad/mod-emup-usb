#!/bin/sh
# MOD-EMUP USB adapter: schematic, routed PCB, checks and fab files from
# emup_adapter.py (the board description), with the generator in
# ../tools/kicad (KiCad 8 + Freerouting; see ../tools/kicad/README.md).
#   emup_adapter.kicad_sch / .kicad_pcb / .pdf   the design
#   fab/                                         Gerbers + drill (zip), BOM
set -e
cd "$(dirname "$0")"
KICAD=${KICAD:-/Applications/KiCad8/KiCad.app/Contents}
KP=$KICAD/Frameworks/Python.framework/Versions/3.9/bin/python3
CLI=$KICAD/MacOS/kicad-cli
T=../tools/kicad
python3 $T/gen_sch.py emup_adapter.py emup_adapter.kicad_sch
$CLI sch erc -o erc.txt emup_adapter.kicad_sch >/dev/null || true
$KP $T/gen_pcb.py emup_adapter.py emup_adapter.kicad_sch emup_adapter.kicad_pcb
# power nets on 1 mm tracks (gen_pcb writes a fresh project file)
python3 - <<'PY'
import json
p = "emup_adapter.kicad_pro"
d = json.load(open(p))
n = json.load(open("netclasses.json"))
ns = d.setdefault("net_settings", {})
ns["classes"] = [c for c in ns.get("classes", []) if c["name"] != "Power"] + n["classes"]
ns["netclass_patterns"] = n["netclass_patterns"]
json.dump(d, open(p, "w"), indent=2)
PY
$KP $T/route.py emup_adapter.kicad_pcb 200
$CLI pcb drc -o drc.txt emup_adapter.kicad_pcb >/dev/null || true
grep -E "Found" drc.txt
$CLI sch export pdf -o emup_adapter.pdf emup_adapter.kicad_sch >/dev/null
rm -rf fab && mkdir -p fab/gerber
$CLI pcb export gerbers -o fab/gerber/ emup_adapter.kicad_pcb >/dev/null
$CLI pcb export drill -o fab/gerber/ emup_adapter.kicad_pcb >/dev/null
(cd fab/gerber && zip -q ../emup_adapter_gerbers.zip *) && rm -rf fab/gerber
$CLI sch export bom -o fab/bom.csv --fields "Reference,Value,Footprint" emup_adapter.kicad_sch >/dev/null 2>&1 || true
echo "fab/: $(ls fab)"
