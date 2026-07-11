# Auto Water — Project Context

## Overview

Automated plant watering system using an ESP32-WROOM microcontroller to control a solenoid valve (or water pump) via a 5V relay, with a web interface for manual control.

## Hardware Design

### Components

| Component | Part | Purpose |
|-----------|------|---------|
| MCU | ESP32-WROOM (WiFi dev board) | Controller + web server |
| Relay | 5V DC coil, 0.36W (72mA), 5-pin SPDT | Switches water valve/pump |
| Transistor | 2N2222A (NPN) | Drives relay coil (GPIO can't source 72mA) |
| Resistor | 1kΩ | Limits base current from GPIO (~2mA) |
| Diode | 1N4007 | Flyback protection across relay coil |
| Connector J1 | 2-pin | ESP32 GPIO4 + GND input |
| Connector J2 | 2-pin | Load output (solenoid valve / pump) |

### Circuit Topology

```
ESP32 GPIO4 → [1kΩ] → Base (2N2222A)
                        Emitter → GND
                        Collector → Relay COIL-

ESP32 5V pin → Relay COIL+
               Flyback diode (1N4007) across coil (cathode to 5V, anode to collector)

Relay NO → Load terminal 1 (J2 pin 1)
Relay COM → Load terminal 2 (J2 pin 2)
```

### Power

- ESP32 dev board powered via USB (5V)
- 5V rail from USB/VIN pin powers the relay coil
- 3.3V GPIO drives transistor base through 1kΩ (draws ~2mA from GPIO, well within limits)
- Total relay coil draw: 72mA from 5V rail — within USB budget

### KiCad Files

- `auto-water.kicad_pro` — Project file
- `auto-water.kicad_sch` — Schematic (relay driver circuit)
- PCB layout not yet started

## Software Design

### Platform

- **Language:** Rust
- **Framework:** `esp-idf-svc` / `esp-idf-hal` (std, runs on FreeRTOS)
- **Target:** `xtensa-esp32-espidf`
- **Toolchain:** Installed via `espup`

### Architecture

- ESP32 connects to WiFi on boot
- Runs an HTTP server (`EspHttpServer` from `esp-idf-svc`)
- Web UI exposes on/off control for the relay (GPIO4)
- Future: scheduling, soil moisture sensor input, multiple zones

### Key Crates

| Crate | Version (approx) | Purpose |
|-------|-------------------|---------|
| `esp-idf-svc` | 0.49+ | WiFi, HTTP server, NVS |
| `esp-idf-hal` | 0.44+ | GPIO, peripherals |
| `anyhow` | 1 | Error handling |
| `embuild` | 0.32+ | Build system integration |

### Web Server Endpoints

| Method | Path | Action |
|--------|------|--------|
| GET | `/` | Status page with ON/OFF buttons |
| GET | `/on` | Set GPIO4 HIGH → relay activates → valve opens |
| GET | `/off` | Set GPIO4 LOW → relay deactivates → valve closes |

### GPIO Pin Assignment

| GPIO | Function |
|------|----------|
| GPIO4 | Relay control (output, drives transistor base via 1kΩ) |

## Design Decisions

1. **`esp-idf-svc` (std) over `esp-hal` (no_std):** Chose std approach because WiFi support is mature and we get threads, `String`, `Vec`, etc. FreeRTOS runs underneath.

2. **Transistor driver over direct GPIO:** GPIO4 outputs 3.3V at ~12mA max. Relay coil needs 5V at 72mA. NPN transistor acts as a level-shifting current amplifier.

3. **KiCad for schematic/PCB:** Open-source, text-based S-expression format, git-friendly.

4. **Relay over MOSFET direct drive:** Using a mechanical relay to switch the load (solenoid valve). Provides galvanic isolation between control and load circuits. Load could be 12V/24V DC or even mains AC valve.

## TODO

- [ ] Scaffold Rust firmware project (cargo generate esp-idf-template)
- [ ] Implement WiFi connection with credentials from NVS
- [ ] Implement HTTP server with relay control
- [ ] Add scheduling (time-based watering)
- [ ] Add soil moisture sensor (ADC input)
- [ ] PCB layout in KiCad
- [ ] 3D-printed enclosure
- [ ] OTA firmware updates
