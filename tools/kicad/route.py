#!/usr/bin/env python3
"""Autoroute a KiCad PCB with Freerouting, then put back and fill its pours.

Run with KiCad's own Python (it has pcbnew):
  .../Python.framework/Versions/3.9/bin/python3 route.py board.kicad_pcb [passes]

1. (this process) note the copper pours, strip pours and tracks, export
   board.dsn. The pours come off so GND gets real traces too (Freerouting
   leaves a poured net's pads to the pour, which dense routing chops into
   islands).
2. Freerouting, headless (tools/freerouting/*.jar; Homebrew's openjdk).
3. (a fresh process: pcbnew's Python objects go bad after deletions)
   import board.ses, re-add the pours, fill, save.
"""
import glob
import shutil
import json
import os
import subprocess
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
JAVA = os.environ.get("JAVA") or shutil.which("java") or "/opt/homebrew/opt/openjdk/bin/java"
JAR = sorted(glob.glob(os.path.join(HERE, "..", "freerouting", "freerouting-*.jar")))[-1]


def stage1(path, passes):
    base = os.path.splitext(path)[0]
    board = pcbnew.LoadBoard(path)
    zones = []
    for i in range(board.GetAreaCount()):
        z = board.GetArea(i)
        o = z.Outline()
        zones.append({"layer": z.GetLayer(), "net": z.GetNetname(), "thick": z.GetMinThickness(),
                      "pts": [[o.CVertex(k).x, o.CVertex(k).y] for k in range(o.TotalVertices())]})
    json.dump(zones, open(base + ".zones.json", "w"))
    items = [board.GetArea(i) for i in range(board.GetAreaCount())] + list(board.GetTracks())
    for it in items:
        board.Remove(it)
    if not pcbnew.ExportSpecctraDSN(board, base + ".dsn"):
        raise SystemExit("DSN export failed")
    board.Save(base + ".tmp.kicad_pcb")
    if os.path.exists(base + ".ses"):
        os.remove(base + ".ses")
    r = subprocess.run([JAVA, "-Djava.awt.headless=true", "-Dapple.awt.UIElement=true", "-jar", JAR, "-de", base + ".dsn", "-do", base + ".ses", "-mp", passes,
                        "--gui.enabled=false"], capture_output=True, text=True, timeout=1800)
    if not os.path.exists(base + ".ses"):
        print(r.stdout[-3000:], r.stderr[-3000:])
        raise SystemExit("Freerouting produced no session")
    subprocess.run([sys.executable, __file__, "--stage2", path], check=True)


def stage2(path):
    base = os.path.splitext(path)[0]
    board = pcbnew.LoadBoard(base + ".tmp.kicad_pcb")
    if not pcbnew.ImportSpecctraSES(board, base + ".ses"):
        raise SystemExit("SES import failed")
    for zd in json.load(open(base + ".zones.json")):
        z = pcbnew.ZONE(board)
        z.SetLayer(zd["layer"])
        z.SetNet(board.FindNet(zd["net"]))
        z.SetLocalClearance(pcbnew.FromMM(0.4))
        z.SetMinThickness(zd["thick"])
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
        o = z.Outline()
        o.NewOutline()
        for x, y in zd["pts"]:
            o.Append(x, y)
        board.Add(z)
    pcbnew.ZONE_FILLER(board).Fill(board.Zones())
    board.Save(path)
    for f in (".dsn", ".ses", ".zones.json", ".tmp.kicad_pcb"):
        os.remove(base + f)
    print("%s: %d tracks/vias" % (path, len(board.GetTracks())))


if __name__ == "__main__":
    if sys.argv[1] == "--stage2":
        stage2(sys.argv[2])
    else:
        stage1(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else "40")
