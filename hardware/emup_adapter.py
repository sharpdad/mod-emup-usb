# MOD-EMUP USB adapter: replaces the SAC-201 ISA card. An Arduino Nano (on
# USB) drives the box's data bus and strobes through a 74HCT245 / 74HCT244;
# one 12 V supply powers the box: +12 V straight through, +5 V (2 A) from a
# RECOM R-78B switching module, -12 V from an isolated TRACO TMR module
# with its +output tied to ground. All through-hole.
#
# Board description for tools/kicad (gen_sch.py / gen_pcb.py / route.py).
# DB25 pinout: Tom Trebisky's cable page (see ../README.md); check it with
# a meter against your own card and box before powering anything.
TITLE = "MOD-EMUP USB adapter"

DIP20S = "Package_DIP:DIP-20_W7.62mm_Socket"
RES = "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P10.16mm_Horizontal"
CAP = "Capacitor_THT:C_Disc_D5.0mm_W2.5mm_P5.00mm"
CP8 = "Capacitor_THT:CP_Radial_D8.0mm_P3.50mm"
CP6 = "Capacitor_THT:CP_Radial_D6.3mm_P2.50mm"


def cap(ref, value, fp, a, b):
    lib = "Device:C_Polarized" if fp != CAP else "Device:C"
    return Part(ref, lib, value, fp, {"#1": a, "#2": b})


# The box's DB25 (as the card's): signals and supplies
DB25 = {1: "GND", 2: "GND", 3: "+5V", 4: "-12V", 5: "BD0", 6: "BD2", 7: "BD4", 8: "BD6",
        9: "~RDDAT", 10: "~WRID", 11: "+5V", 12: "GND", 13: "GND", 14: "GND", 15: "+5V",
        16: "+12V", 17: "~RESET", 18: "BD1", 19: "BD3", 20: "BD5", 21: "BD7", 22: "~WRDAT",
        23: "+12V", 24: "+5V", 25: "GND"}

NANO = {  # Arduino Nano pin numbers (KiCad's Arduino_Nano_v2.x)
    "D2": 5, "D3": 6, "D4": 7, "D5": 8, "D6": 9, "D7": 10, "D8": 11, "D9": 12, "D10": 13,
    "A0": 19, "A1": 20, "A2": 21, "A3": 22, "GND": 4, "GND2": 29, "+5V": 27}

PARTS = [
    Part("J1", "Connector:Barrel_Jack", "12 V DC in (2.1 mm, centre +)", "Connector_BarrelJack:BarrelJack_Horizontal",
         {"#1": "VIN", "#2": "GND"}),
    Part("D1", "Device:D_Schottky", "1N5822 (reverse protection)", "Diode_THT:D_DO-201AD_P15.24mm_Horizontal",
         {"#1": "+12V", "#2": "VIN"}),
    Part("U1", "Regulator_Linear:L7805", "RECOM R-78B5.0-2.0", "Converter_DCDC:Converter_DCDC_RECOM_R-78B-2.0_THT",
         {"#1": "+12V", "#2": "GND", "#3": "+5V"}),
    Part("U2", "Converter_DCDC:TMR-1211", "TRACO TMR 1-1211 (-12 V: +out to GND)",
         "Converter_DCDC:Converter_DCDC_TRACO_TMR-1-xxxx_Single_THT",
         {"#1": "GND", "#2": "+12V", "#6": "GND", "#7": "-12V"}),
    Part("A1", "MCU_Module:Arduino_Nano_v3.x", "Arduino Nano (5 V)", "Module:Arduino_Nano",
         dict([("#%d" % NANO["D%d" % (i + 2)], "ND%d" % i) for i in range(8)] +
              [("#%d" % NANO["D10"], "DIR"), ("#%d" % NANO["A0"], "N_WRID"), ("#%d" % NANO["A1"], "N_WRDAT"),
               ("#%d" % NANO["A2"], "N_RDDAT"), ("#%d" % NANO["A3"], "N_RESET"),
               ("#%d" % NANO["GND"], "GND"), ("#%d" % NANO["GND2"], "GND")])),
    Part("U3", "74xx:74HC245", "74HCT245", DIP20S,
         dict([("#%d" % (2 + i), "ND%d" % i) for i in range(8)] +
              [("#%d" % (18 - i), "BD%d" % i) for i in range(8)] +
              [("#1", "DIR"), ("#19", "GND"), ("#20", "+5V"), ("#10", "GND")])),
    Part("U4", "74xx:74HC244", "74HCT244", DIP20S,
         {"#2": "N_WRID", "#4": "N_WRDAT", "#6": "N_RDDAT", "#8": "N_RESET",
          "#18": "~WRID", "#16": "~WRDAT", "#14": "~RDDAT", "#12": "~RESET",
          "#11": "GND", "#13": "GND", "#15": "GND", "#17": "GND",          # (unused half: inputs tied)
          "#1": "GND", "#19": "GND", "#20": "+5V", "#10": "GND"}),
    Part("J2", "Connector:DB25_Receptacle", "DB25 female (to the MOD-EMUP box)",
         "Connector_Dsub:DSUB-25_Female_Horizontal_P2.77x2.84mm_EdgePinOffset14.56mm_Housed_MountingHolesOffset15.98mm",
         {"#%d" % p: n for p, n in DB25.items()}),
    # strobes idle (high) while the Nano boots or is unplugged
    Part("R1", "Device:R", "10k", RES, {"#1": "N_WRID", "#2": "+5V"}),
    Part("R2", "Device:R", "10k", RES, {"#1": "N_WRDAT", "#2": "+5V"}),
    Part("R3", "Device:R", "10k", RES, {"#1": "N_RDDAT", "#2": "+5V"}),
    Part("R4", "Device:R", "10k", RES, {"#1": "N_RESET", "#2": "+5V"}),
    Part("R5", "Device:R", "1k", RES, {"#1": "+5V", "#2": "LED"}),
    Part("D2", "Device:LED", "LED (+5 V on)", "LED_THT:LED_D3.0mm", {"#1": "GND", "#2": "LED"}),
    cap("C1", "100u 25V", CP8, "+12V", "GND"),
    cap("C2", "220u 10V", CP8, "+5V", "GND"),
    cap("C3", "47u 25V", CP6, "GND", "-12V"),          # (its + to ground: the rail is negative)
    cap("C4", "100n", CAP, "+5V", "GND"),
    cap("C5", "100n", CAP, "+5V", "GND"),
    Part("FLG2", "power:PWR_FLAG", "PWR_FLAG", "", {"#1": "VIN"}),
    Part("FLG5", "power:PWR_FLAG", "PWR_FLAG", "", {"#1": "+12V"}),
]

BOARD = (120, 80)
PLACE = {
    "J1": (10, 14, 0), "D1": (25, 8, 0), "C1": (48, 8, 0),
    "U1": (58, 6, 0), "C2": (74, 8, 0), "U2": (86, 6, 0), "C3": (108, 8, 0),
    "A1": (8, 40, 0),
    "U3": (36, 26, 0), "U4": (54, 26, 0), "C4": (36, 54, 0), "C5": (54, 54, 0),
    "R1": (72, 26, 0), "R2": (72, 30, 0), "R3": (72, 34, 0), "R4": (72, 38, 0),
    "R5": (72, 44, 0), "D2": (90, 44, 0),
    "J2": (98, 72, 0),
}
