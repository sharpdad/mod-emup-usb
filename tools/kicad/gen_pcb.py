#!/usr/bin/env python3
"""Make an unrouted KiCad 8 PCB from a generated schematic: footprints placed
per the card's PLACE table, nets on every pad, a board outline, and GND
copper pours on both layers. Run with KiCad's own Python (it has pcbnew):

  /Applications/KiCad8/KiCad.app/Contents/Frameworks/Python.framework/Versions/3.9/bin/python3 \
      gen_pcb.py card.py card.kicad_sch card.kicad_pcb

The card description supplies PLACE = {ref: (x_mm, y_mm, rotation_deg)} and
BOARD = (width_mm, height_mm).
"""
import os
import re
import subprocess
import sys

import pcbnew

KICAD = "/Applications/KiCad8/KiCad.app/Contents"
FP = KICAD + "/SharedSupport/footprints"


def netlist(sch):
    out = "/tmp/_gen_pcb.net"
    subprocess.run([KICAD + "/MacOS/kicad-cli", "sch", "export", "netlist", "--format", "kicadsexpr",
                    "-o", out, sch], check=True, capture_output=True)
    t = open(out).read()
    comps = {}
    for m in re.finditer(r'\(comp \(ref "([^"]+)"\)\s*\(value "([^"]*)"\)\s*\(footprint "([^"]*)"\)', t):
        comps[m.group(1)] = (m.group(2), m.group(3))
    nets = {}
    for blk in t.split("(net (code")[1:]:
        name = re.search(r'\(name "([^"]*)"', blk).group(1).lstrip("/")
        for ref, pin in re.findall(r'\(node \(ref "([^"]+)"\) \(pin "([^"]+)"', blk):
            nets[(ref, pin)] = name
    return comps, nets


def main(card, sch, out):
    ns = {"Part": lambda *a, **k: None}
    here = os.getcwd()
    os.chdir(os.path.dirname(os.path.abspath(card)))
    exec(open(os.path.basename(card)).read(), ns)
    os.chdir(here)
    place, (bw, bh) = ns["PLACE"], ns["BOARD"]
    comps, nets = netlist(sch)
    board = pcbnew.BOARD()
    netinfo = {}

    def net(name):
        if name not in netinfo:
            ni = pcbnew.NETINFO_ITEM(board, name)
            board.Add(ni)
            netinfo[name] = ni
        return netinfo[name]

    mm = pcbnew.FromMM
    for ref, (value, fpid) in sorted(comps.items()):
        if not fpid:
            continue                                # (power flags)
        lib, name = fpid.split(":")
        fp = pcbnew.FootprintLoad(os.path.join(FP, lib + ".pretty"), name)
        fp.SetReference(ref)
        fp.SetValue(value)
        x, y, rot = place.get(ref, (bw + 10, 10, 0))
        board.Add(fp)
        fp.SetPosition(pcbnew.VECTOR2I(mm(x), mm(y)))
        fp.SetOrientationDegrees(rot)
        for pad in fp.Pads():
            n = nets.get((ref, pad.GetNumber()))
            if n and not n.startswith("unconnected"):
                pad.SetNet(net(n))
        if ref not in place:
            print("  (not placed: %s)" % ref)
    # outline
    pts = [(0, 0), (bw, 0), (bw, bh), (0, bh)]
    for i in range(4):
        seg = pcbnew.PCB_SHAPE(board)
        seg.SetShape(pcbnew.SHAPE_T_SEGMENT)
        seg.SetLayer(pcbnew.Edge_Cuts)
        seg.SetWidth(mm(0.1))
        seg.SetStart(pcbnew.VECTOR2I(mm(pts[i][0]), mm(pts[i][1])))
        seg.SetEnd(pcbnew.VECTOR2I(mm(pts[(i + 1) % 4][0]), mm(pts[(i + 1) % 4][1])))
        board.Add(seg)
    # GND pours, both sides
    for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
        z = pcbnew.ZONE(board)
        z.SetLayer(layer)
        z.SetNet(net("GND"))
        z.SetLocalClearance(mm(0.4))
        z.SetMinThickness(mm(0.25))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
        o = z.Outline()
        o.NewOutline()
        for (x, y) in [(0.5, 0.5), (bw - 0.5, 0.5), (bw - 0.5, bh - 0.5), (0.5, bh - 0.5)]:
            o.Append(mm(x), mm(y))
        board.Add(z)
    # design rules: generous for hobby fabs
    ds = board.GetDesignSettings()
    ds.m_TrackMinWidth = mm(0.2)
    ds.m_MinClearance = mm(0.2)
    nc = ds.m_NetSettings.m_DefaultNetClass
    nc.SetTrackWidth(mm(0.3))
    nc.SetClearance(mm(0.25))
    nc.SetViaDiameter(mm(0.8))
    nc.SetViaDrill(mm(0.4))
    board.Save(out)
    print("%s: %d footprints, %d nets, %.0f x %.0f mm" % (out, len([c for c in comps.values() if c[1]]),
                                                      len(netinfo), bw, bh))


if __name__ == "__main__":
    main(*sys.argv[1:4])
