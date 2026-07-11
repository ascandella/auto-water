# Auto Water

Automated plant watering system: ESP32-WROOM + SSR + solenoid valve, controlled via a web interface.

## Hardware

ESP32 drives a solid state relay through an NPN transistor (2N2222A), switching a solenoid valve or water pump. Schematic in KiCad format.

## Software

Rust firmware using `esp-idf-svc` (std). Hosts a web server on the local network for on/off control.

## Getting Started

### Hardware
1. Open `auto-water.kicad_sch` in [KiCad](https://www.kicad.org/) to view/edit the schematic
2. Wire per the schematic: GPIO26 → 1kΩ → 2N2222A base, 5V → SSR input

### Firmware
```bash
# Install toolchain
cargo install espup
espup install
source ~/export-esp.sh

# Build and flash
cargo run
```

## Documentation

See [docs/CONTEXT.md](docs/CONTEXT.md) for full project context, design decisions, and TODO list.
