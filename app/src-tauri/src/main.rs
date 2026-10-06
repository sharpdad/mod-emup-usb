// EMUP Studio: a modern front end for the MOD-EMUP / ALL-03 universal
// programmer (emup/README.md). The window is plain HTML (../ui); this side
// does what the page can't: files, the serial port to the USB adapter, and
// the programming operations.
//
// Until the adapter exists, operations run against a simulated chip (one
// buffer per device size), so the whole interface can be used now. The
// real path will send command lists (write ID, write data, read, delay) to
// the adapter, which times the pulses itself.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod emup;

use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, State};

const DEVICES: &str = include_str!("../devices.json");

/// A loaded or read image.
#[derive(Serialize)]
struct Image {
    format: String,
    bytes: Vec<u8>,
    fuses: Option<String>, // JEDEC: the fuse map as '0'/'1'
    sum16: u16,            // byte sum, as on Atari's ROM labels
    crc32: u32,
    note: String,
}

#[derive(Serialize, Clone)]
struct Progress {
    op: String,
    done: usize,
    total: usize,
}

#[derive(Serialize)]
struct OpResult {
    ok: bool,
    message: String,
    image: Option<Image>,
}

/// The simulated chip in the socket (stands in for the box until the
/// adapter is built). Starts blank: all FF.
struct Sim {
    chip: Mutex<Vec<u8>>,
}

fn crc32(d: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFF_FFFF;
    for &b in d {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn image(format: &str, bytes: Vec<u8>, fuses: Option<String>, note: String) -> Image {
    let sum16 = bytes.iter().fold(0u16, |s, &b| s.wrapping_add(b as u16));
    let crc = crc32(&bytes);
    Image { format: format.into(), bytes, fuses, sum16, crc32: crc, note }
}

/// Intel HEX: data records into a buffer starting at the lowest address.
fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let mut recs: Vec<(u32, Vec<u8>)> = vec![];
    let mut base: u32 = 0;
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let h = l.strip_prefix(':').ok_or(format!("line {}: no ':'", n + 1))?;
        let b: Vec<u8> = (0..h.len() / 2)
            .map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16))
            .collect::<Result<_, _>>()
            .map_err(|_| format!("line {}: not hex", n + 1))?;
        if b.len() < 5 || b.iter().fold(0u8, |s, &x| s.wrapping_add(x)) != 0 {
            return Err(format!("line {}: bad length or checksum", n + 1));
        }
        let len = b[0] as usize;
        let addr = (b[1] as u32) << 8 | b[2] as u32;
        match b[3] {
            0 => recs.push((base + addr, b[4..4 + len].to_vec())),
            1 => break,
            2 => base = ((b[4] as u32) << 8 | b[5] as u32) << 4,
            4 => base = ((b[4] as u32) << 8 | b[5] as u32) << 16,
            _ => {}
        }
    }
    let lo = recs.iter().map(|r| r.0).min().unwrap_or(0);
    let hi = recs.iter().map(|r| r.0 + r.1.len() as u32).max().unwrap_or(0);
    let mut out = vec![0xFFu8; (hi - lo) as usize];
    for (a, d) in recs {
        let o = (a - lo) as usize;
        out[o..o + d.len()].copy_from_slice(&d);
    }
    Ok(out)
}

/// JEDEC: the fuse map (L fields), default from F.
fn parse_jedec(text: &str) -> Result<(String, usize), String> {
    let body: String = text.chars().filter(|&c| c != '\x02' && c != '\x03').collect();
    let mut qf = 0usize;
    let mut default = '0';
    let mut fields = vec![];
    for f in body.split('*').skip(1) {
        let f = f.trim();
        if let Some(r) = f.strip_prefix("QF") {
            qf = r.trim().parse().map_err(|_| "bad QF")?;
        } else if let Some(r) = f.strip_prefix('F') {
            default = if r.trim().starts_with('1') { '1' } else { '0' };
        } else if let Some(r) = f.strip_prefix('L') {
            fields.push(r.to_string());
        }
    }
    if qf == 0 {
        return Err("no QF (fuse count) field".into());
    }
    let mut map: Vec<char> = vec![default; qf];
    for f in fields {
        let mut it = f.split_whitespace();
        let start: usize = it.next().and_then(|s| s.parse().ok()).ok_or("bad L field")?;
        let bits: String = it.collect::<String>();
        for (i, c) in bits.chars().enumerate() {
            if start + i < qf && (c == '0' || c == '1') {
                map[start + i] = c;
            }
        }
    }
    Ok((map.into_iter().collect(), qf))
}

#[tauri::command]
fn devices() -> serde_json::Value {
    serde_json::from_str(DEVICES).unwrap_or(serde_json::Value::Null)
}

/// Parse a file the page read (it sends the name and the bytes).
#[tauri::command]
fn parse_file(name: String, data: Vec<u8>) -> Result<Image, String> {
    let lower = name.to_lowercase();
    let text = String::from_utf8_lossy(&data);
    if lower.ends_with(".jed") || text.trim_start_matches('\x02').trim_start().starts_with("JEDEC")
        || (text.contains("*QF") && text.contains("*L"))
    {
        let (fuses, n) = parse_jedec(&text)?;
        let bytes: Vec<u8> = fuses
            .as_bytes()
            .chunks(8)
            .map(|c| c.iter().enumerate().fold(0u8, |b, (i, &f)| b | (((f == b'1') as u8) << i)))
            .collect();
        return Ok(image("JEDEC", bytes, Some(fuses), format!("{} fuses", n)));
    }
    if lower.ends_with(".hex") || lower.ends_with(".ihx") || text.starts_with(':') {
        let b = parse_hex(&text)?;
        let n = b.len();
        return Ok(image("Intel HEX", b, None, format!("{} bytes", n)));
    }
    let n = data.len();
    Ok(image("Binary", data, None, format!("{} bytes", n)))
}

/// Serial ports (the adapter will be one of them).
#[tauri::command]
fn ports() -> Vec<String> {
    serialport::available_ports()
        .map(|v| v.into_iter().map(|p| p.port_name).collect())
        .unwrap_or_default()
}

/// Run an operation on the chip: read, blank, program, verify. `size` is
/// the device's byte count; `data` the buffer (program / verify).
#[tauri::command]
async fn run(
    app: tauri::AppHandle,
    sim: State<'_, Sim>,
    op: String,
    size: usize,
    data: Option<Vec<u8>>,
    device: Option<String>,
    port: Option<String>,
) -> Result<OpResult, String> {
    // Chips with real algorithms (emup.rs): through the box (the model with a
    // virtual chip, or the USB adapter on a serial port).
    if let Some(chip) = device.as_deref().and_then(emup::Eprom::by_name) {
        let mem = {
            let mut m = sim.chip.lock().unwrap();
            if m.len() != chip.size { *m = vec![0xFF; chip.size]; }
            m.clone()
        };
        let app2 = app.clone();
        let op2 = op.clone();
        let port2 = port.clone();
        let dev = device.clone().unwrap_or_default();
        let (res, mem) = tauri::async_runtime::spawn_blocking(move || box_op(&app2, &op2, &dev, chip, mem, data, port2))
            .await
            .map_err(|e| e.to_string())?;
        if let Some(m) = mem { *sim.chip.lock().unwrap() = m; }
        return res;
    }
    let size = size.max(1);
    {
        let mut chip = sim.chip.lock().unwrap();
        if chip.len() != size {
            *chip = vec![0xFF; size];
        }
    }
    // pace like a real part: bipolar PROMs and EPROMs program byte by byte
    let per = Duration::from_micros(match op.as_str() {
        "program" => (1_500_000 / size as u64).max(40),
        _ => (600_000 / size as u64).max(10),
    });
    let step = (size / 100).max(1);
    let mut bad: Option<(usize, u8, u8)> = None;
    for i in 0..size {
        {
            let mut chip = sim.chip.lock().unwrap();
            match op.as_str() {
                "program" => {
                    let want = data.as_ref().and_then(|d| d.get(i)).copied().unwrap_or(0xFF);
                    // fuses only go one way: bits can be programmed 1 -> 0
                    chip[i] &= want;
                    if chip[i] != want && bad.is_none() {
                        bad = Some((i, want, chip[i]));
                    }
                }
                "verify" => {
                    let want = data.as_ref().and_then(|d| d.get(i)).copied().unwrap_or(0xFF);
                    if chip[i] != want && bad.is_none() {
                        bad = Some((i, want, chip[i]));
                    }
                }
                "blank" => {
                    if chip[i] != 0xFF && bad.is_none() {
                        bad = Some((i, 0xFF, chip[i]));
                    }
                }
                _ => {}
            }
        }
        if i % step == 0 || i + 1 == size {
            let _ = app.emit("progress", Progress { op: op.clone(), done: i + 1, total: size });
            tokio_sleep(per * step as u32).await;
        }
    }
    let chip = sim.chip.lock().unwrap().clone();
    let (ok, message) = match (op.as_str(), bad) {
        ("read", _) => (true, format!("Read {} bytes.", size)),
        ("blank", None) => (true, "Blank.".into()),
        ("blank", Some((a, _, g))) => (false, format!("Not blank: {:04X} = {:02X}.", a, g)),
        ("program", None) => (true, "Programmed and verified.".into()),
        ("program", Some((a, w, g))) => (false, format!("Failed at {:04X}: wanted {:02X}, got {:02X} (a bit already blown?).", a, w, g)),
        ("verify", None) => (true, "Verified: chip matches the buffer.".into()),
        ("verify", Some((a, w, g))) => (false, format!("Mismatch at {:04X}: buffer {:02X}, chip {:02X}.", a, w, g)),
        _ => (false, "Unknown operation.".into()),
    };
    let img = if op == "read" { Some(image("Read from chip", chip, None, format!("{} bytes", size))) } else { None };
    Ok(OpResult { ok, message, image: img })
}

/// One operation through emup.rs. Returns the result and, on the model,
/// the virtual chip's new contents.
fn box_op(app: &tauri::AppHandle, op: &str, dev: &str, chip: emup::Eprom, mem: Vec<u8>, data: Option<Vec<u8>>,
          port: Option<String>) -> (Result<OpResult, String>, Option<Vec<u8>>) {
    let size = chip.size;
    let name = op.to_string();
    let mut progress = |done: usize, total: usize| {
        let _ = app.emit("progress", Progress { op: name.clone(), done, total });
    };
    let want = data.unwrap_or_default();
    let result = |bus: &mut dyn BusOps, progress: &mut dyn FnMut(usize, usize)| -> Result<OpResult, String> {
        match op {
            "read" => {
                let d = bus.read_all(progress)?;
                Ok(OpResult { ok: true, message: format!("Read {} bytes.", d.len()),
                              image: Some(image("Read from chip", d, None, format!("{} bytes", size))) })
            }
            "blank" => {
                let d = bus.read_all(progress)?;
                Ok(match d.iter().position(|&b| b != 0xFF) {
                    None => OpResult { ok: true, message: "Blank.".into(), image: None },
                    Some(a) => OpResult { ok: false, message: format!("Not blank: {:04X} = {:02X}.", a, d[a]), image: None },
                })
            }
            "program" => {
                let mut w = want.clone();
                w.resize(size, 0xFF);
                bus.program(&w, progress)?;
                Ok(OpResult { ok: true, message: "Programmed and verified.".into(), image: None })
            }
            "verify" => {
                let d = bus.read_all(progress)?;
                let mut w = want.clone();
                w.resize(size, 0xFF);
                Ok(match d.iter().zip(&w).position(|(g, x)| g != x) {
                    None => OpResult { ok: true, message: "Verified: chip matches the buffer.".into(), image: None },
                    Some(a) => OpResult { ok: false, message: format!("Mismatch at {:04X}: buffer {:02X}, chip {:02X}.", a, w[a], d[a]), image: None },
                })
            }
            _ => Err("unknown operation".into()),
        }
    };
    match port.as_deref() {
        None | Some("sim") => {
            let mut b = Boxed { bus: emup::SimBox::new(Some((chip, mem))), chip: emup::Eprom::by_name(dev) };
            let r = result(&mut b, &mut progress);
            let mem = b.bus.chip.map(|c| c.1);
            (r, mem)
        }
        Some(p) => match emup::SerialBus::open(p) {
            Ok(bus) => {
                let mut b = Boxed { bus, chip: emup::Eprom::by_name(dev) };
                (result(&mut b, &mut progress), None)
            }
            Err(e) => (Err(e), None),
        },
    }
}

/// An algorithm runner over some Bus, for box_op.
trait BusOps {
    fn read_all(&mut self, p: &mut dyn FnMut(usize, usize)) -> Result<Vec<u8>, String>;
    fn program(&mut self, d: &[u8], p: &mut dyn FnMut(usize, usize)) -> Result<(), String>;
}

struct Boxed<B: emup::Bus> {
    bus: B,
    chip: Option<emup::Eprom>,
}

impl<B: emup::Bus> BusOps for Boxed<B> {
    fn read_all(&mut self, p: &mut dyn FnMut(usize, usize)) -> Result<Vec<u8>, String> {
        let c = self.chip.as_ref().ok_or("no chip")?;
        emup::read(&mut self.bus, c, p)
    }
    fn program(&mut self, d: &[u8], p: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
        let c = self.chip.as_ref().ok_or("no chip")?;
        emup::program(&mut self.bus, c, d, p)
    }
}

async fn tokio_sleep(d: Duration) {
    // (no async runtime dependency of our own: Tauri's blocking pool)
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

/// Swap in a fresh (blank) simulated chip.
#[tauri::command]
fn new_chip(sim: State<'_, Sim>) {
    sim.chip.lock().unwrap().iter_mut().for_each(|b| *b = 0xFF);
}

fn main() {
    tauri::Builder::default()
        .manage(Sim { chip: Mutex::new(vec![]) })
        .invoke_handler(tauri::generate_handler![devices, parse_file, ports, run, new_chip])
        .run(tauri::generate_context!())
        .expect("error while running EMUP Studio");
}
