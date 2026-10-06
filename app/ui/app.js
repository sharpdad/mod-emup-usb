// EMUP Studio window. Talks to the Rust side through Tauri's invoke/listen;
// opened in a plain browser (for a quick look) it falls back to a small
// in-page stand-in.
const T = window.__TAURI__;
const $ = (id) => document.getElementById(id);

const api = T ? {
  devices: () => T.core.invoke("devices"),
  parse: (name, data) => T.core.invoke("parse_file", { name, data: Array.from(data) }),
  ports: () => T.core.invoke("ports"),
  run: (op, size, data, device, port) => T.core.invoke("run", { op, size, data: data ? Array.from(data) : null, device, port }),
  onProgress: (f) => T.event.listen("progress", (e) => f(e.payload)),
} : browserStandIn();

let devices = [], dev = null, buf = null;   // buf: {bytes: Uint8Array, title, sum16, crc32, fuses}

// ---- chips ---------------------------------------------------------------
function renderList() {
  const q = $("search").value.trim().toLowerCase();
  const list = $("devlist");
  list.innerHTML = "";
  let fam = null;
  for (const d of devices) {
    const hay = (d.name + " " + d.maker + " " + d.family + " " + d.note).toLowerCase();
    if (q && !hay.includes(q)) continue;
    if (d.family !== fam) {
      fam = d.family;
      const h = document.createElement("div");
      h.className = "fam"; h.textContent = fam;
      list.appendChild(h);
    }
    const b = document.createElement("button");
    b.className = "dev"; b.setAttribute("role", "option");
    b.setAttribute("aria-selected", dev && dev.name === d.name);
    b.innerHTML = `<span class="n">${d.name}</span><span class="o">${d.org}</span><span class="m">${d.maker} · ${d.pins}-pin</span>`;
    b.onclick = () => pick(d);
    list.appendChild(b);
  }
}

function pick(d) {
  dev = d;
  $("dev-family").textContent = d.family;
  $("dev-name").textContent = d.name;
  const facts = [d.org, `${d.pins}-pin DIP`, d.maker, `DOS: ${d.dos}`];
  $("dev-facts").innerHTML = facts.map((f) => `<span>${f}</span>`).join("") +
    (d.note ? `<span>${d.note}</span>` : "");
  drawSocket(d.pins);
  renderList();
  refreshButtons();
}

// ---- the socket ------------------------------------------------------------
// 40-pin ZIF, lever at the top. Chips sit at the bottom (their last pins in
// socket rows 20/21), notch toward the lever, as the ALL-03 manual shows.
function drawSocket(pins) {
  const svg = $("socket");
  const rows = 20, top = 70, pitch = 19, lx = 60, rx = 160;
  let s = `<rect x="30" y="20" width="160" height="${top + rows * pitch - 4}" rx="8" fill="var(--zif)"/>`;
  s += `<rect x="168" y="4" width="10" height="60" rx="4" fill="var(--pin)"/><text x="110" y="48" text-anchor="middle" font-size="11" fill="var(--pin)" font-family="var(--sans)">lever</text>`;
  const first = pins ? rows - pins / 2 : rows;          // chip rows: first..19
  for (let r = 0; r < rows; r++) {
    const y = top + r * pitch;
    for (const x of [lx, rx]) s += `<rect x="${x - 6}" y="${y - 4}" width="12" height="8" rx="2" fill="var(--zif-hole)"/>`;
  }
  if (pins) {
    const y0 = top + first * pitch - 12, h = (pins / 2) * pitch + 4;
    s += `<rect x="${lx - 10}" y="${y0}" width="${rx - lx + 20}" height="${h}" rx="4" fill="var(--chip)" stroke="var(--accent)" stroke-width="2"/>`;
    s += `<path d="M ${(lx + rx) / 2 - 10} ${y0} a 10 10 0 0 0 20 0" fill="var(--zif)"/>`;
    s += `<circle cx="${lx + 12}" cy="${y0 + 14}" r="4" fill="var(--accent)"/>`;
    s += `<text x="${(lx + rx) / 2}" y="${y0 + h / 2 + 5}" text-anchor="middle" font-size="15" font-weight="600" fill="var(--pin)" font-family="var(--mono)">${dev.name}</text>`;
    s += `<text x="${lx - 16}" y="${y0 + 18}" text-anchor="end" font-size="11" fill="var(--accent)" font-family="var(--mono)">1</text>`;
    $("socket-cap").textContent = `Pin 1 (dot) at socket pin ${first + 1}, notch toward the lever; the chip sits at the bottom of the socket.`;
  } else {
    $("socket-cap").textContent = "The 40-pin socket. Pick a chip to see where it goes.";
  }
  s += `<text x="${lx - 16}" y="${top + 4}" text-anchor="end" font-size="10" fill="var(--muted)" font-family="var(--mono)">1</text>`;
  s += `<text x="${lx - 16}" y="${top + 19 * pitch + 4}" text-anchor="end" font-size="10" fill="var(--muted)" font-family="var(--mono)">20</text>`;
  svg.innerHTML = s;
}

// ---- the buffer --------------------------------------------------------------
const ROWH = 20;
function setBuffer(img, title) {
  buf = { bytes: Uint8Array.from(img.bytes), title, sum16: img.sum16, crc32: img.crc32, fuses: img.fuses };
  $("buf-title").textContent = `${title} · ${img.format} · ${img.note}`;
  $("buf-sums").innerHTML = `<span><b>sum16</b>${hex(img.sum16, 4)}</span><span><b>CRC32</b>${hex(img.crc32 >>> 0, 8)}</span>` +
    (dev && dev.bytes && img.bytes.length !== dev.bytes ? `<span style="color:var(--bad)">size ≠ ${dev.bytes}</span>` : "");
  $("hex-spacer").style.height = Math.ceil(buf.bytes.length / 16) * ROWH + "px";
  $("drop").scrollTop = 0;
  renderHex();
  refreshButtons();
}

function hex(v, n) { return v.toString(16).toUpperCase().padStart(n, "0"); }

function renderHex() {
  const view = $("drop"), out = $("hex-rows");
  if (!buf) { out.innerHTML = ""; return; }
  const first = Math.floor(view.scrollTop / ROWH), n = Math.ceil(view.clientHeight / ROWH) + 2;
  const total = Math.ceil(buf.bytes.length / 16);
  let h = "";
  for (let r = first; r < Math.min(total, first + n); r++) {
    const a = r * 16, row = buf.bytes.subarray(a, a + 16);
    let hx = "", asc = "";
    row.forEach((b, i) => {
      hx += `<span class="${b === 0xFF ? "ff" : ""}">${hex(b, 2)}</span>` + (i === 7 ? "  " : " ");
      asc += b >= 32 && b < 127 ? String.fromCharCode(b).replace(/[<&]/g, (c) => c === "<" ? "&lt;" : "&amp;") : "·";
    });
    h += `<div class="row" style="position:absolute;top:${r * ROWH}px"><span class="a">${hex(a, 5)}</span>  ${hx} <span class="d">${asc}</span></div>`;
  }
  out.innerHTML = h;
}

async function loadFile(file) {
  const data = new Uint8Array(await file.arrayBuffer());
  try {
    setBuffer(await api.parse(file.name, data), file.name);
    log(`Loaded ${file.name}.`, "ok");
  } catch (e) { log(`${file.name}: ${e}`, "bad"); }
}

// ---- operations -------------------------------------------------------------
let busy = false;
function refreshButtons() {
  const has = !!dev, size = dev ? (dev.bytes || Math.ceil((dev.fuses || 0) / 8)) : 0;
  for (const b of document.querySelectorAll(".buttons button")) {
    const needsBuf = b.dataset.op === "program" || b.dataset.op === "verify";
    b.disabled = busy || !has || !size || (needsBuf && !buf);
  }
}

async function op(name) {
  if (!dev) return;
  const size = dev.bytes || Math.ceil((dev.fuses || 0) / 8);
  busy = true; refreshButtons();
  const label = { read: "Reading", blank: "Blank checking", program: "Programming", verify: "Verifying" }[name];
  $("progress-label").textContent = `${label} ${dev.name}…`;
  try {
    const r = await api.run(name, size, buf && (name === "program" || name === "verify") ? buf.bytes : null, dev.name, $("port").value);
    log(`${dev.name}: ${r.message}`, r.ok ? "ok" : "bad");
    $("progress-label").textContent = r.message;
    if (r.image) setBuffer(r.image, `Read from ${dev.name}`);
  } catch (e) {
    log(`${dev.name}: ${e}`, "bad");
  }
  busy = false; refreshButtons();
}

function log(msg, cls) {
  const d = document.createElement("div");
  d.className = cls || "";
  d.textContent = `${new Date().toLocaleTimeString()}  ${msg}`;
  $("log").prepend(d);
}

// ---- wiring ---------------------------------------------------------------
async function main() {
  devices = await api.devices();
  renderList();
  drawSocket(0);
  const ports = await api.ports();
  const sel = $("port");
  sel.innerHTML = `<option value="sim">Simulated box (no adapter yet)</option>` +
    ports.map((p) => `<option value="${p}">${p}</option>`).join("");
  $("conn-note").textContent = (ports.length ? `${ports.length} serial port(s) found; pick the USB adapter. ` : "No adapter yet. ") +
    "2716 and 2732 run the real register-level algorithms on a model of the box; other chips use a simple stand-in.";
  $("search").addEventListener("input", renderList);
  $("file").addEventListener("change", (e) => e.target.files[0] && loadFile(e.target.files[0]));
  const drop = $("drop");
  drop.addEventListener("scroll", renderHex);
  window.addEventListener("resize", renderHex);
  drop.addEventListener("dragover", (e) => { e.preventDefault(); drop.classList.add("drag"); });
  drop.addEventListener("dragleave", () => drop.classList.remove("drag"));
  drop.addEventListener("drop", (e) => { e.preventDefault(); drop.classList.remove("drag"); e.dataTransfer.files[0] && loadFile(e.dataTransfer.files[0]); });
  for (const b of document.querySelectorAll(".buttons button")) b.onclick = () => op(b.dataset.op);
  api.onProgress((p) => {
    $("bar-fill").style.width = (100 * p.done / p.total).toFixed(1) + "%";
    $("progress-label").textContent = `${p.op} ${p.done} / ${p.total}`;
  });
  refreshButtons();
  log("Ready. Pick a chip, load a file, or read a chip.");
}
main();

// ---- outside the app: a small stand-in so the page can be looked at -------------
function browserStandIn() {
  let chip = null, prog = () => {};
  const sum = (d) => d.reduce((s, b) => (s + b) & 0xFFFF, 0);
  return {
    devices: async () => [
      { name: "82S129", maker: "Signetics", family: "Bipolar PROM", pins: 16, org: "256 x 4", bytes: 256, dos: "BPPGM", note: "Tempest Math Box." },
      { name: "2716", maker: "Intel", family: "EPROM", pins: 24, org: "2K x 8", bytes: 2048, dos: "EPP512", note: "25 V." },
      { name: "GAL22V10", maker: "Lattice", family: "GAL", pins: 24, org: "5892 fuses", bytes: 0, fuses: 5892, dos: "GAL1", note: "" },
    ],
    parse: async (name, data) => ({ format: "Binary", bytes: Array.from(data), sum16: sum(data), crc32: 0, note: `${data.length} bytes` }),
    ports: async () => [],
    onProgress: (f) => { prog = f; },
    run: async (op, size, data) => {
      if (!chip || chip.length !== size) chip = new Uint8Array(size).fill(0xFF);
      for (let i = 0; i <= 20; i++) { prog({ op, done: Math.round(size * i / 20), total: size }); await new Promise((r) => setTimeout(r, 40)); }
      if (op === "program") chip.forEach((_, i) => chip[i] &= data[i] ?? 0xFF);
      const img = op === "read" ? { format: "Read", bytes: Array.from(chip), sum16: sum(chip), crc32: 0, note: `${size} bytes` } : null;
      return { ok: true, message: `${op} done (browser stand-in).`, image: img };
    },
  };
}
