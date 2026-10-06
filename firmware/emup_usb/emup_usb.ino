// MOD-EMUP USB adapter: an Arduino (5 V) in place of the SAC-201 ISA card.
//
// The box needs only what the ISA card gave it: an 8-bit data bus, three
// active-low strobes (write ID port, write data port, read data port) and
// /RESET, plus power (+5 V, +12 V, -12 V from a separate supply, NOT from
// the Arduino). See emup/README.md for the DB25 pinout. Put a 74LS245 on
// the data bus and a 74LS244 on the strobes, as the card had.
//
// Host protocol (USB serial, 1,000,000 baud): a batch is
//   'B' len_lo len_hi  <len bytes of ops>
// ops:
//   0x01 id         write the ID port
//   0x02 v          write the data port
//   0x03            read the data port (one byte back)
//   0x04 id v       ID then data (the usual register write)
//   0x05 id         ID then read (one byte back)
//   0x06 lo hi      wait lo + 256 hi microseconds
//   0x07 lo hi      wait lo + 256 hi milliseconds
//   0x08            pulse /RESET (the box clears every register)
// The adapter runs the whole batch, so programming pulses are timed here,
// not across USB, then replies 'K' + the bytes read, in order. 'V' replies
// "EMUP1\n". Batches are capped at 4 KB.
//
// Pins (Arduino Nano / Uno; change to suit):
//   D2-D9   data bus D0-D7 (through the 74LS245)
//   D10     245 direction (high: Arduino -> box)
//   A0      /WRID   (write ID port, DB25 pin 10)
//   A1      /WRDAT  (write data port, DB25 pin 22)
//   A2      /RDDAT  (read data port, DB25 pin 9)
//   A3      /RESET  (DB25 pin 17)

const uint8_t DATA_PINS[8] = {2, 3, 4, 5, 6, 7, 8, 9};
const uint8_t DIR = 10;
const uint8_t WRID = A0, WRDAT = A1, RDDAT = A2, RST = A3;

static uint8_t batch[4096];

static void busOut(uint8_t v) {
  digitalWrite(DIR, HIGH);
  for (uint8_t i = 0; i < 8; i++) {
    pinMode(DATA_PINS[i], OUTPUT);
    digitalWrite(DATA_PINS[i], (v >> i) & 1);
  }
}

static uint8_t busIn() {
  for (uint8_t i = 0; i < 8; i++) pinMode(DATA_PINS[i], INPUT);
  digitalWrite(DIR, LOW);
  return 0;
}

static void strobe(uint8_t pin) {
  digitalWrite(pin, LOW);
  delayMicroseconds(1);           // the ISA cycle was ~1 us
  digitalWrite(pin, HIGH);
}

static void writeId(uint8_t id) { busOut(id); strobe(WRID); }
static void writeData(uint8_t v) { busOut(v); strobe(WRDAT); }

static uint8_t readData() {
  busIn();
  digitalWrite(RDDAT, LOW);
  delayMicroseconds(1);
  uint8_t v = 0;
  for (uint8_t i = 0; i < 8; i++) v |= digitalRead(DATA_PINS[i]) << i;
  digitalWrite(RDDAT, HIGH);
  return v;
}

static int readByte() {
  while (!Serial.available()) {}
  return Serial.read();
}

void setup() {
  for (uint8_t p : {WRID, WRDAT, RDDAT, RST}) { pinMode(p, OUTPUT); digitalWrite(p, HIGH); }
  pinMode(DIR, OUTPUT);
  busIn();
  Serial.begin(1000000);
}

void loop() {
  int c = readByte();
  if (c == 'V') { Serial.print("EMUP1\n"); return; }
  if (c != 'B') return;
  uint16_t len = readByte();
  len |= (uint16_t)readByte() << 8;
  if (len > sizeof batch) len = sizeof batch;
  for (uint16_t i = 0; i < len; i++) batch[i] = readByte();
  Serial.write('K');
  for (uint16_t i = 0; i < len;) {
    switch (batch[i++]) {
      case 0x01: writeId(batch[i++]); break;
      case 0x02: writeData(batch[i++]); break;
      case 0x03: Serial.write(readData()); break;
      case 0x04: writeId(batch[i]); writeData(batch[i + 1]); i += 2; break;
      case 0x05: writeId(batch[i++]); Serial.write(readData()); break;
      case 0x06: { uint16_t us = batch[i] | (batch[i + 1] << 8); i += 2;
                   while (us > 16000) { delayMicroseconds(16000); us -= 16000; }
                   delayMicroseconds(us); break; }
      case 0x07: { uint16_t ms = batch[i] | (batch[i + 1] << 8); i += 2; delay(ms); break; }
      case 0x08: digitalWrite(RST, LOW); delay(1); digitalWrite(RST, HIGH); delay(1); break;
      default: i = len; break;    // unknown op: stop
    }
  }
}
