#!/usr/bin/env python3
"""Generate a KiCad 8 schematic from a parts + connections description.

A card is described in Python (see multigame/hw/*.py):
    PARTS = [Part("U1", "Memory_Flash:SST39SF040", "SST39SF040", footprint,
                  {"A0": "A0", ...}), ...]
Connections are by pin name or pin number ("#12"). Every pin of every unit
is placed and gets a net label at its end (or a no-connect flag), so the
schematic is plain to read: one block per part, nets by name. Symbols are
copied (flattened) from KiCad's own libraries.

Usage: gen_sch.py card.py out.kicad_sch
"""
import os
import re
import sys
import uuid

KICAD = "/Applications/KiCad8/KiCad.app/Contents/SharedSupport/symbols"


# ------------------------------------------------------------ s-expressions
def parse(text):
    tok = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()]+', text)
    pos = 0

    def rd():
        nonlocal pos
        t = tok[pos]
        pos += 1
        if t == "(":
            out = []
            while tok[pos] != ")":
                out.append(rd())
            pos += 1
            return out
        return t
    return rd()


def dump(x, ind=0):
    if not isinstance(x, list):
        return x
    if all(not isinstance(e, list) for e in x):
        return "(" + " ".join(x) + ")"
    s = "(" + x[0]
    for e in x[1:]:
        s += ("\n" + "  " * (ind + 1) + dump(e, ind + 1)) if isinstance(e, list) else " " + e
    return s + ")"


def q(s):
    return '"' + s.replace('"', '\\"') + '"'


def unq(s):
    return s[1:-1] if s.startswith('"') else s


_libs = {}


def lib_symbol(lib_id):
    """The flattened symbol (extends resolved) as an s-expression list."""
    lib, name = lib_id.split(":")
    if lib not in _libs:
        _libs[lib] = parse(open(os.path.join(KICAD, lib + ".kicad_sym")).read())
    syms = {unq(s[1]): s for s in _libs[lib] if isinstance(s, list) and s[0] == "symbol"}
    s = [e for e in syms[name]]
    ext = [e for e in s if isinstance(e, list) and e[0] == "extends"]
    if ext:
        parent = lib_symbol(lib + ":" + unq(ext[0][1]))
        pname = unq(parent[1]).split(":")[1]
        props = {unq(e[1]): e for e in s if isinstance(e, list) and e[0] == "property"}
        out = [s[0], q(name)]
        for e in parent[2:]:
            if isinstance(e, list) and e[0] == "property" and unq(e[1]) in props:
                out.append(props.pop(unq(e[1])))
            elif isinstance(e, list) and e[0] == "symbol":
                out.append([e[0], q(unq(e[1]).replace(pname, name, 1))] + e[2:])
            else:
                out.append(e)
        out[2:2] = list(props.values())
        s = out
    s = [s[0], q(lib_id)] + s[2:]
    return s


def pins_of(sym):
    """[(unit, number, name, x, y, angle, hidden)] from a flattened symbol."""
    out = []
    for sub in sym:
        if not (isinstance(sub, list) and sub[0] == "symbol"):
            continue
        m = re.match(r'.*_(\d+)_(\d+)$', unq(sub[1]))
        unit = int(m.group(1))
        if int(m.group(2)) > 1:
            continue                        # (De Morgan alternate body)
        for p in sub:
            if isinstance(p, list) and p[0] == "pin":
                at = [e for e in p if isinstance(e, list) and e[0] == "at"][0]
                nm = [e for e in p if isinstance(e, list) and e[0] == "name"][0]
                nb = [e for e in p if isinstance(e, list) and e[0] == "number"][0]
                hidden = "hide" in p
                out.append((unit, unq(nb[1]), unq(nm[1]), float(at[1]), float(at[2]), int(float(at[3])), hidden))
    return out


def units_of(sym):
    us = set()
    for sub in sym:
        if isinstance(sub, list) and sub[0] == "symbol":
            us.add(int(re.match(r'.*_(\d+)_\d+$', unq(sub[1])).group(1)))
    us.discard(0)
    return sorted(us) or [1]


def bbox(sym, unit):
    xs, ys = [0.0], [0.0]
    for sub in sym:
        if isinstance(sub, list) and sub[0] == "symbol":
            m = re.match(r'.*_(\d+)_(\d+)$', unq(sub[1]))
            u = int(m.group(1))
            if u not in (0, unit) or int(m.group(2)) > 1:
                continue
            for p in sub:
                if isinstance(p, list) and p[0] == "pin":
                    at = [e for e in p if isinstance(e, list) and e[0] == "at"][0]
                    xs.append(float(at[1])); ys.append(float(at[2]))
    return min(xs), max(xs), min(ys), max(ys)


# ------------------------------------------------------------------- model
class Part:
    def __init__(self, ref, lib_id, value, footprint, conn, nc_ok=True):
        self.ref, self.lib_id, self.value, self.footprint = ref, lib_id, value, footprint
        self.conn = conn            # pin name or "#number" -> net
        self.nc_ok = nc_ok


def U():
    return str(uuid.uuid4())


def generate(title, parts, out_path):
    root = U()
    syms = {}
    for p in parts:
        if p.lib_id not in syms:
            syms[p.lib_id] = lib_symbol(p.lib_id)
    items = []
    netpins = {}
    x0, y0, rowh, x, y = 30.48, 30.48, 0.0, 30.48, 30.48
    MAXW = 380.0
    for p in parts:
        sym = syms[p.lib_id]
        pins = pins_of(sym)
        used = set()
        for unit in units_of(sym):
            mnx, mxx, mny, mxy = bbox(sym, unit)
            w, h = (mxx - mnx) + 40.64, (mxy - mny) + 20.32
            if x + w > MAXW:
                x, y, rowh = x0, y + rowh, 0.0
            ox = round((x - mnx + 15.24) / 2.54) * 2.54
            oy = round((y + mxy + 10.16) / 2.54) * 2.54
            x += w
            rowh = max(rowh, h)
            inst = [p.ref]
            props = [
                '(property "Reference" %s (at %.2f %.2f 0) (effects (font (size 1.27 1.27))))' % (q(p.ref), ox, oy - mxy - 3.81),
                '(property "Value" %s (at %.2f %.2f 0) (effects (font (size 1.27 1.27))))' % (q(p.value), ox, oy - mny + 3.81),
                '(property "Footprint" %s (at %.2f %.2f 0) (effects (font (size 1.27 1.27)) hide))' % (q(p.footprint), ox, oy),
                '(property "Datasheet" "" (at %.2f %.2f 0) (effects (font (size 1.27 1.27)) hide))' % (ox, oy),
            ]
            upins = [pp for pp in pins if pp[0] in (0, unit)]
            pinlines = " ".join('(pin %s (uuid %s))' % (q(pp[1]), U()) for pp in upins)
            items.append('(symbol (lib_id %s) (at %.2f %.2f 0) (unit %d) (exclude_from_sim no) (in_bom yes) (on_board yes) (dnp no) (uuid %s)\n  %s\n  %s\n  (instances (project %s (path "/%s" (reference %s) (unit %d)))))'
                         % (q(p.lib_id), ox, oy, unit, U(), "\n  ".join(props), pinlines, q(title), root, q(p.ref), unit))
            for (pu, num, name, px, py, ang, hidden) in upins:
                if pu == 0 and unit != units_of(sym)[0]:
                    continue                    # common pins: once
                net = p.conn.get("#" + num, p.conn.get(name))
                used.add("#" + num); used.add(name)
                sx, sy = ox + px, oy - py
                if net is None:
                    if not p.nc_ok:
                        raise SystemExit("%s pin %s (%s) unconnected" % (p.ref, num, name))
                    items.append('(no_connect (at %.2f %.2f) (uuid %s))' % (sx, sy, U()))
                    continue
                la = {0: 180, 180: 0, 90: 270, 270: 90}[ang]
                just = "right" if la in (180, 270) else "left"
                items.append('(label %s (at %.2f %.2f %d) (fields_autoplaced yes) (effects (font (size 1.27 1.27)) (justify %s bottom)) (uuid %s))'
                             % (q(net), sx, sy, la, just, U()))
                netpins.setdefault(net, []).append("%s.%s" % (p.ref, num))
        missing = [k for k in p.conn if k not in used]
        if missing:
            raise SystemExit("%s: no such pin(s) %s" % (p.ref, missing))
    libsyms = "\n".join(dump(s, 1) for s in syms.values())
    text = ('(kicad_sch (version 20231120) (generator "gen_sch.py") (generator_version "8.0") (uuid %s) (paper "A3")\n'
            '(title_block (title %s))\n(lib_symbols\n%s)\n%s\n(sheet_instances (path "/" (page "1"))))\n'
            % (root, q(title), libsyms, "\n".join(items)))
    open(out_path, "w").write(text)
    return netpins


if __name__ == "__main__":
    ns = {"Part": Part}
    exec(open(sys.argv[1]).read(), ns)
    netpins = generate(ns["TITLE"], ns["PARTS"], sys.argv[2])
    single = sorted(n for n, ps in netpins.items() if len(ps) < 2)
    print("%s: %d parts, %d nets" % (sys.argv[2], len(ns["PARTS"]), len(netpins)))
    if single:
        print("  nets with one pin:", ", ".join(single))
