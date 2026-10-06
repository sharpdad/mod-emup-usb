// The MOD-EMUP (ALL-03) box, its registers, and chip algorithms.
//
// Registers (emup/ref/all03info.txt): write an ID to the ID port, then
// read or write the data port. E0-E4 TTL I/O, socket pins 1-40 (open
// collector: write 1 before reading or before applying a supply); E5 VOP
// DAC (0.1 V/step), E6 VHH (0.06 V), E7 VCC (0.04 V); E8-EC VOP enable per
// pin; ED/EE VCC enable (a fixed pin list); EF ground select (b0 low: pin
// 20 grounded); F7 b3 the GOOD lamp. Supplies drop ~0.6 V to the socket.
//
// `Bus` is what an algorithm talks to: the USB adapter (command batches,
// see emup/firmware/emup_usb) or `SimBox`, a model of the box with a
// virtual chip in the socket, so the algorithms can be developed now.

pub const TTL: u8 = 0xE0;
pub const VOP_DAC: u8 = 0xE5;
pub const VCC_DAC: u8 = 0xE7;
pub const VOP_EN: u8 = 0xE8;
pub const VCC_EN: u8 = 0xED;
pub const OTHER: u8 = 0xEF;
pub const LAMP: u8 = 0xF7;

/// VCC enable bits: (register, bit) for the socket pins that have one.
const VCC_PINS: [(u8, u8, u8); 14] = [
    (40, 0, 0), (36, 0, 1), (34, 0, 2), (32, 0, 3), (31, 0, 4), (30, 0, 5), (29, 0, 6), (28, 0, 7),
    (27, 1, 0), (26, 1, 1), (9, 1, 2), (7, 1, 3), (5, 1, 4), (1, 1, 5),
];

pub trait Bus {
    fn write(&mut self, id: u8, v: u8);
    /// Queue a read; its value comes back from `flush`, in order.
    fn read(&mut self, id: u8);
    fn delay_us(&mut self, us: u32);
    fn flush(&mut self) -> Result<Vec<u8>, String>;
}

/// A chip on the socket: socket pin for chip pin k (1-based), chips sit at
/// the bottom of the 40-pin socket (ALL-03 convention).
pub fn socket_pin(pins: u8, k: u8) -> u8 {
    k + 20 - pins / 2
}

/// 27xx EPROM pin roles (chip pin numbers).
pub struct Eprom {
    pub pins: u8,
    pub size: usize,
    pub addr: Vec<u8>, // chip pins of A0, A1, ...
    pub data: [u8; 8], // chip pins of D0..D7
    pub ce: u8,
    pub oe: u8,
    pub vcc: u8,
    pub gnd: u8,
    pub vpp: Option<u8>, // a separate VPP pin (2716: 21)
    pub prog_volts: f32,
}

impl Eprom {
    pub fn by_name(name: &str) -> Option<Eprom> {
        let a24 = vec![8, 7, 6, 5, 4, 3, 2, 1, 23, 22, 19];
        match name {
            "2716" => Some(Eprom { pins: 24, size: 2048, addr: a24, data: [9, 10, 11, 13, 14, 15, 16, 17],
                                    ce: 18, oe: 20, vcc: 24, gnd: 12, vpp: Some(21), prog_volts: 25.0 }),
            "2732" => {
                let mut a = a24;
                a.push(21);
                Some(Eprom { pins: 24, size: 4096, addr: a, data: [9, 10, 11, 13, 14, 15, 16, 17],
                             ce: 18, oe: 20, vcc: 24, gnd: 12, vpp: None, prog_volts: 21.0 })
            }
            _ => None,
        }
    }
}

/// Register image for TTL pins: 5 bytes, bit = socket pin - 1.
#[derive(Clone)]
pub struct Pins([u8; 5]);

impl Pins {
    pub fn all_high() -> Pins { Pins([0xFF; 5]) }
    pub fn set(&mut self, socket: u8, high: bool) {
        let (r, b) = (((socket - 1) / 8) as usize, (socket - 1) % 8);
        if high { self.0[r] |= 1 << b } else { self.0[r] &= !(1 << b) }
    }
    pub fn get(raw: &[u8], socket: u8) -> bool {
        raw[((socket - 1) / 8) as usize] >> ((socket - 1) % 8) & 1 != 0
    }
}

fn vcc_enable_bits(sockets: &[u8]) -> [u8; 2] {
    let mut r = [0u8; 2];
    for &(p, reg, bit) in VCC_PINS.iter() {
        if sockets.contains(&p) { r[reg as usize] |= 1 << bit; }
    }
    r
}

/// Power the chip for reading: TTL all released, VCC 5 V on its VCC pin
/// (and VPP), its ground pin grounded (pin 20 only, for now).
pub fn power_up<B: Bus>(bus: &mut B, c: &Eprom) -> Result<(), String> {
    let gnd = socket_pin(c.pins, c.gnd);
    if gnd != 20 {
        return Err(format!("chip ground lands on socket pin {}; only pin 20 is supported so far", gnd));
    }
    for i in 0..5 { bus.write(TTL + i, 0xFF); }
    bus.write(OTHER, 0x00);                         // pin 20 to ground
    bus.write(VCC_DAC, ((5.0 + 0.6) / 0.04) as u8); // 5 V at the socket
    let mut on = vec![socket_pin(c.pins, c.vcc)];
    if let Some(v) = c.vpp { on.push(socket_pin(c.pins, v)); }
    let en = vcc_enable_bits(&on);
    bus.write(VCC_EN, en[0]);
    bus.write(VCC_EN + 1, en[1]);
    bus.delay_us(10_000);
    Ok(())
}

pub fn power_down<B: Bus>(bus: &mut B) {
    bus.write(VCC_EN, 0);
    bus.write(VCC_EN + 1, 0);
    for i in 0..5 { bus.write(VOP_EN + i, 0); }
    bus.write(VCC_DAC, 0);
    bus.write(VOP_DAC, 0);
}

fn address_pins(c: &Eprom, a: usize, ce_low: bool, oe_low: bool) -> Pins {
    let mut p = Pins::all_high();
    for (i, &pin) in c.addr.iter().enumerate() {
        p.set(socket_pin(c.pins, pin), a >> i & 1 != 0);
    }
    p.set(socket_pin(c.pins, c.ce), !ce_low);
    p.set(socket_pin(c.pins, c.oe), !oe_low);
    p
}

/// Read the whole chip; `progress(done, total)` after each batch.
pub fn read<B: Bus>(bus: &mut B, c: &Eprom, progress: &mut dyn FnMut(usize, usize)) -> Result<Vec<u8>, String> {
    power_up(bus, c)?;
    bus.flush()?;
    let mut out = Vec::with_capacity(c.size);
    let chunk = 128;
    for start in (0..c.size).step_by(chunk) {
        for a in start..(start + chunk).min(c.size) {
            let p = address_pins(c, a, true, true);
            for i in 0..5 { bus.write(TTL + i, p.0[i as usize]); }
            for i in 0..5 { bus.read(TTL + i); }
        }
        let raw = bus.flush()?;
        for r in raw.chunks(5) {
            let mut v = 0u8;
            for (bit, &pin) in c.data.iter().enumerate() {
                if Pins::get(r, socket_pin(c.pins, pin)) { v |= 1 << bit; }
            }
            out.push(v);
        }
        progress(out.len(), c.size);
    }
    power_down(bus);
    bus.flush()?;
    Ok(out)
}

/// 2716-style programming: VPP at prog_volts on its pin, /OE high, data on
/// the data pins, a 50 ms high pulse on /CE (PGM) per byte; then verify.
pub fn program<B: Bus>(bus: &mut B, c: &Eprom, data: &[u8], progress: &mut dyn FnMut(usize, usize)) -> Result<(), String> {
    let vpp = c.vpp.ok_or("this chip's programming isn't written yet")?;
    power_up(bus, c)?;
    // VPP: TTL already released on that pin; VCC off it, VOP on
    let en = vcc_enable_bits(&[socket_pin(c.pins, c.vcc)]);
    bus.write(VCC_EN, en[0]);
    bus.write(VCC_EN + 1, en[1]);
    bus.write(VOP_DAC, ((c.prog_volts + 0.6) / 0.1) as u8);
    let vp = socket_pin(c.pins, vpp);
    let mut vop = [0u8; 5];
    vop[((vp - 1) / 8) as usize] |= 1 << ((vp - 1) % 8);
    for i in 0..5 { bus.write(VOP_EN + i, vop[i as usize]); }
    bus.delay_us(10_000);
    bus.flush()?;
    for (a, &v) in data.iter().enumerate().take(c.size) {
        if v == 0xFF { continue; }                  // erased already
        let mut p = address_pins(c, a, false, false);
        p.set(socket_pin(c.pins, c.ce), false);     // PGM low
        p.set(socket_pin(c.pins, c.oe), true);      // /OE high
        for (bit, &pin) in c.data.iter().enumerate() { p.set(socket_pin(c.pins, pin), v >> bit & 1 != 0); }
        for i in 0..5 { bus.write(TTL + i, p.0[i as usize]); }
        p.set(socket_pin(c.pins, c.ce), true);      // PGM high: the 50 ms pulse
        bus.write(TTL + (socket_pin(c.pins, c.ce) - 1) / 8, p.0[((socket_pin(c.pins, c.ce) - 1) / 8) as usize]);
        bus.delay_us(50_000);
        p.set(socket_pin(c.pins, c.ce), false);
        bus.write(TTL + (socket_pin(c.pins, c.ce) - 1) / 8, p.0[((socket_pin(c.pins, c.ce) - 1) / 8) as usize]);
        if a % 16 == 15 {
            bus.flush()?;
            progress(a + 1, c.size);
        }
    }
    power_down(bus);
    bus.flush()?;
    let back = read(bus, c, &mut |_, _| {})?;
    match back.iter().zip(data).position(|(g, w)| g != w) {
        None => Ok(()),
        Some(a) => Err(format!("verify failed at {:04X}: wanted {:02X}, got {:02X}", a, data[a], back[a])),
    }
}

// ---------------------------------------------------------------------------
// A model of the box with a virtual EPROM in the socket.

pub struct SimBox {
    regs: [u8; 256],
    queue: Vec<u8>,
    now_us: u64,
    pgm_rose_at: Option<u64>,
    pub chip: Option<(Eprom, Vec<u8>)>,
    pub log: Vec<String>,
}

impl SimBox {
    pub fn new(chip: Option<(Eprom, Vec<u8>)>) -> SimBox {
        SimBox { regs: [0; 256], queue: vec![], now_us: 0, pgm_rose_at: None, chip, log: vec![] }
    }

    fn vcc_on(&self, socket: u8) -> bool {
        VCC_PINS.iter().any(|&(p, r, b)| p == socket && self.regs[(VCC_EN + r) as usize] >> b & 1 != 0)
    }
    fn vop_on(&self, socket: u8) -> bool {
        self.regs[(VOP_EN + (socket - 1) / 8) as usize] >> ((socket - 1) % 8) & 1 != 0
    }
    fn ttl(&self, socket: u8) -> bool { Pins::get(&self.regs[TTL as usize..TTL as usize + 5], socket) }

    /// Is the virtual chip powered and grounded correctly?
    fn powered(&self) -> bool {
        let Some((c, _)) = &self.chip else { return false };
        let vcc = self.regs[VCC_DAC as usize] as f32 * 0.04 - 0.6;
        self.vcc_on(socket_pin(c.pins, c.vcc)) && (vcc - 5.0).abs() < 0.3
            && socket_pin(c.pins, c.gnd) == 20 && self.regs[OTHER as usize] & 1 == 0
            && self.ttl(socket_pin(c.pins, c.vcc))     // (a low TTL on a supply pin would short it)
    }

    fn addr(&self) -> usize {
        let (c, _) = self.chip.as_ref().unwrap();
        c.addr.iter().enumerate().fold(0, |a, (i, &p)| a | ((self.ttl(socket_pin(c.pins, p)) as usize) << i))
    }

    /// TTL register value as read back: open collector, so a pin reads low
    /// if the box pulls it low or the chip drives it low.
    fn read_ttl(&self, reg: u8) -> u8 {
        let mut v = self.regs[reg as usize];
        if let Some((c, mem)) = &self.chip {
            let ce = socket_pin(c.pins, c.ce);
            let oe = socket_pin(c.pins, c.oe);
            if self.powered() && !self.ttl(ce) && !self.ttl(oe) {
                let d = mem[self.addr() % c.size];
                for (bit, &pin) in c.data.iter().enumerate() {
                    let s = socket_pin(c.pins, pin);
                    if (s - 1) / 8 == reg - TTL && d >> bit & 1 == 0 {
                        v &= !(1 << ((s - 1) % 8));
                    }
                }
            }
        }
        v
    }

    fn write_reg(&mut self, id: u8, v: u8) {
        let before_pgm = self.pgm_high();
        self.regs[id as usize] = v;
        let after = self.pgm_high();
        if !before_pgm && after {
            self.pgm_rose_at = Some(self.now_us);
        } else if before_pgm && !after {
            if let Some(t0) = self.pgm_rose_at.take() {
                self.pulse(self.now_us - t0);
            }
        }
    }

    fn pgm_high(&self) -> bool {
        match &self.chip {
            Some((c, _)) => self.ttl(socket_pin(c.pins, c.ce)),
            None => false,
        }
    }

    /// A PGM pulse ended: program if VPP is at the right voltage and the
    /// pulse was long enough (2716: 45-55 ms).
    fn pulse(&mut self, us: u64) {
        let vop = self.regs[VOP_DAC as usize] as f32 * 0.1 - 0.6;
        let ok = {
            let Some((c, _)) = &self.chip else { return };
            if c.vpp.is_none() { return; }
            self.powered() || self.vcc_on(socket_pin(c.pins, c.vcc))
        };
        let (c, _) = self.chip.as_ref().unwrap();
        let vpp = socket_pin(c.pins, c.vpp.unwrap());
        let oe = socket_pin(c.pins, c.oe);
        if !ok || !self.vop_on(vpp) || (vop - c.prog_volts).abs() > 1.0 || !self.ttl(oe) {
            self.log.push(format!("pulse ignored: VOP {:.1} V, on={}, /OE high={}", vop, self.vop_on(vpp), self.ttl(oe)));
            return;
        }
        if !(45_000..=55_000).contains(&us) {
            self.log.push(format!("pulse {} us: out of spec, ignored", us));
            return;
        }
        let a = self.addr() % c.size;
        let mut v = 0u8;
        for (bit, &pin) in c.data.iter().enumerate() {
            if self.ttl(socket_pin(c.pins, pin)) { v |= 1 << bit; }
        }
        let (_, mem) = self.chip.as_mut().unwrap();
        mem[a] &= v;                                 // EPROM cells only go 1 -> 0
    }
}

impl Bus for SimBox {
    fn write(&mut self, id: u8, v: u8) { self.write_reg(id, v); }
    fn read(&mut self, id: u8) {
        let v = if (TTL..TTL + 5).contains(&id) { self.read_ttl(id) } else { self.regs[id as usize] };
        self.queue.push(v);
    }
    fn delay_us(&mut self, us: u32) { self.now_us += us as u64; }
    fn flush(&mut self) -> Result<Vec<u8>, String> { Ok(std::mem::take(&mut self.queue)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_back_a_2716() {
        let data: Vec<u8> = (0..2048).map(|i| (i * 7 + 3) as u8).collect();
        let mut b = SimBox::new(Some((Eprom::by_name("2716").unwrap(), data.clone())));
        let got = read(&mut b, &Eprom::by_name("2716").unwrap(), &mut |_, _| {}).unwrap();
        assert_eq!(got, data);
    }

    #[test]
    fn program_a_blank_2716() {
        let want: Vec<u8> = (0..2048).map(|i| (i ^ 0x5A) as u8).collect();
        let mut b = SimBox::new(Some((Eprom::by_name("2716").unwrap(), vec![0xFF; 2048])));
        program(&mut b, &Eprom::by_name("2716").unwrap(), &want, &mut |_, _| {}).unwrap();
        assert_eq!(b.chip.as_ref().unwrap().1, want, "log: {:?}", b.log);
    }

    #[test]
    fn a_2716_sits_at_socket_9() {
        assert_eq!(socket_pin(24, 1), 9);
        assert_eq!(socket_pin(24, 12), 20);
        assert_eq!(socket_pin(24, 24), 32);
    }
}

// ---------------------------------------------------------------------------
// The USB adapter (emup/firmware/emup_usb): operations are batched and the
// adapter runs each batch, timing delays itself.

pub struct SerialBus {
    port: Box<dyn serialport::SerialPort>,
    ops: Vec<u8>,
    reads: usize,
    results: Vec<u8>,
}

impl SerialBus {
    pub fn open(name: &str) -> Result<SerialBus, String> {
        let mut port = serialport::new(name, 1_000_000)
            .timeout(std::time::Duration::from_secs(10))
            .open()
            .map_err(|e| format!("{}: {}", name, e))?;
        std::thread::sleep(std::time::Duration::from_millis(1500));   // (an Arduino resets on open)
        port.write_all(b"V").map_err(|e| e.to_string())?;
        let mut hello = [0u8; 6];
        port.read_exact(&mut hello).map_err(|_| format!("{}: no EMUP adapter answered", name))?;
        if &hello != b"EMUP1\n" {
            return Err(format!("{}: not an EMUP adapter", name));
        }
        Ok(SerialBus { port, ops: vec![], reads: 0, results: vec![] })
    }

    fn send(&mut self) -> Result<(), String> {
        if self.ops.is_empty() { return Ok(()); }
        let n = self.ops.len();
        let mut msg = vec![b'B', n as u8, (n >> 8) as u8];
        msg.extend_from_slice(&self.ops);
        self.port.write_all(&msg).map_err(|e| e.to_string())?;
        let mut back = vec![0u8; 1 + self.reads];
        self.port.read_exact(&mut back).map_err(|e| format!("adapter: {}", e))?;
        if back[0] != b'K' { return Err("adapter: bad reply".into()); }
        self.results.extend_from_slice(&back[1..]);
        self.ops.clear();
        self.reads = 0;
        Ok(())
    }

    fn room(&mut self) {
        if self.ops.len() > 3900 { let _ = self.send(); }
    }
}

impl Bus for SerialBus {
    fn write(&mut self, id: u8, v: u8) { self.ops.extend_from_slice(&[0x04, id, v]); self.room(); }
    fn read(&mut self, id: u8) { self.ops.extend_from_slice(&[0x05, id]); self.reads += 1; self.room(); }
    fn delay_us(&mut self, us: u32) {
        let mut us = us;
        while us > 0 {
            if us >= 60_000 {
                let ms = (us / 1000).min(65_535);
                self.ops.extend_from_slice(&[0x07, ms as u8, (ms >> 8) as u8]);
                us -= ms * 1000;
            } else {
                self.ops.extend_from_slice(&[0x06, us as u8, (us >> 8) as u8]);
                us = 0;
            }
        }
        self.room();
    }
    fn flush(&mut self) -> Result<Vec<u8>, String> {
        self.send()?;
        Ok(std::mem::take(&mut self.results))
    }
}
