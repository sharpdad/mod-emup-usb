/*
 * MOD-EMUP / ALL-03 programmer stand-in, for recording what the DOS
 * programming software does (MOD-EMUP over USB: see README.md).
 *
 * The SAC-201 ISA card has two ports: base+0 (ID port, write: selects a
 * register in the box) and base+2 (data port, read/write). Every access is
 * logged with the emulated time in milliseconds:
 *     <ms> I <id>            ID port write
 *     <ms> W <id> <value>    data port write
 *     <ms> R <id> <value>    data port read (the value returned)
 * The box is simulated as an empty socket with pull-ups: a TTL register
 * (E0-E4) reads back what was last written to it; other registers read
 * their last value.
 *
 * Environment: EMUP_BASE (hex, default 2E0), EMUP_LOG (default emup.log),
 * EMUP_ID (hex: the value register F8, the box's identification, reads),
 * EMUP_SCREEN (if set: the 80 x 25 text screen is written there 4 times a
 * second, so the DOS menus can be followed without a display).
 */
#include "dosbox.h"
#include "inout.h"
#include "pic.h"
#include "logging.h"
#include "mem.h"
#include <stdio.h>
#include <stdlib.h>

static FILE *emup_log = NULL;
static unsigned emup_base = 0x2E0;
static uint8_t emup_id = 0;
static uint8_t emup_reg[256];
static int emup_id_value = -1;          /* EMUP_ID: what register F8 reads */
/* The box's 4-bit counter (found in BPPGM's identification check): writes
 * to E7 with bit 1 low clear it; with bit 1 high, each rising edge of bit 0
 * counts; F8 reads it in the low nibble. */
static uint8_t emup_count = 0;

static void emup_note(char kind, unsigned id, int value) {
    if (!emup_log) return;
    if (value < 0)
        fprintf(emup_log, "%.4f %c %02X\n", (double)PIC_FullIndex(), kind, id);
    else
        fprintf(emup_log, "%.4f %c %02X %02X\n", (double)PIC_FullIndex(), kind, id, value);
}

static void emup_write(Bitu port, Bitu val, Bitu iolen) {
    (void)iolen;
    unsigned off = (unsigned)port - emup_base;
    val &= 0xFF;
    if (off == 0) {
        emup_id = (uint8_t)val;
        emup_note('I', emup_id, -1);
    } else if (off == 2) {
        if (emup_id == 0xE7) {
            uint8_t old = emup_reg[0xE7];
            if ((val & 1) && !(old & 1))        /* a rising clock edge: */
                emup_count = (val & 2) ? (uint8_t)((emup_count + 1) & 0x0F)  /* count */
                                       : 0;                                  /* or clear */
        }
        emup_reg[emup_id] = (uint8_t)val;
        emup_note('W', emup_id, (int)val);
    }
}

static Bitu emup_read(Bitu port, Bitu iolen) {
    (void)iolen;
    unsigned off = (unsigned)port - emup_base;
    if (off == 2) {
        uint8_t v = emup_reg[emup_id];
        if (emup_id == 0xF8) v = emup_id_value >= 0 ? (uint8_t)emup_id_value : emup_count;
        emup_note('R', emup_id, v);
        return v;
    }
    return 0xFF;
}

static const char *emup_screen = NULL;

static void emup_dump_screen(Bitu val) {
    (void)val;
    if (emup_screen) {
        FILE *f = fopen(emup_screen, "w");
        if (f) {
            for (int r = 0; r < 25; r++) {
                char line[81];
                for (int c = 0; c < 80; c++) {
                    uint8_t ch = mem_readb((LinearPt)(0xB8000 + (r * 80 + c) * 2));
                    line[c] = (ch >= 32 && ch < 127) ? (char)ch : (ch ? '#' : ' ');
                }
                line[80] = 0;
                fprintf(f, "%s\n", line);
            }
            fclose(f);
        }
        PIC_AddEvent(emup_dump_screen, 250.0);
    }
}

static void EMUP_ShutDown(Section *sec) {
    (void)sec;
    if (emup_log) {
        fclose(emup_log);
        emup_log = NULL;
    }
}

void EMUP_Init() {
    const char *b = getenv("EMUP_BASE");
    const char *l = getenv("EMUP_LOG");
    if (b) emup_base = (unsigned)strtoul(b, NULL, 16);
    emup_log = fopen(l ? l : "emup.log", "w");
    if (emup_log) setvbuf(emup_log, NULL, _IOLBF, 0);
    for (int i = 0; i < 256; i++) emup_reg[i] = 0;
    IO_RegisterWriteHandler(emup_base, emup_write, IO_MB, 4);
    IO_RegisterReadHandler(emup_base, emup_read, IO_MB, 4);
    const char *idv = getenv("EMUP_ID");
    if (idv) emup_id_value = (int)strtoul(idv, NULL, 16);
    emup_screen = getenv("EMUP_SCREEN");
    if (emup_screen) PIC_AddEvent(emup_dump_screen, 250.0);
    LOG(LOG_MISC, LOG_DEBUG)("EMUP stand-in on ports %xh-%xh", emup_base, emup_base + 3);
    AddExitFunction(AddExitFunctionFuncPair(EMUP_ShutDown), true);
}
