# Changelog

All notable changes to Touch Manager will be documented here. The project follows
[Semantic Versioning](https://semver.org/) once the first stable release is published.

## Unreleased

### Added

- On-demand, checksum-verified downloads for licensed official firmware.
- Content-addressed firmware cache in the application data directory.
- Curated community-directory and upstream-source links opened in the system browser.
- Official firmware redistribution notices.

### Planned

- Signed remote catalog updates.
- Maintained firmware builds with the TouchLink update protocol.
- Signed and notarized release automation.
- Windows and Linux hardware packaging.

## 0.1.0 - 2026-07-10

### Added

- Tauri 2 desktop application with React and TypeScript interface.
- Catalog metadata for 23 Touch 2 firmware releases.
- Binary classification and fixed target-profile validation.
- USB, DFU, and serial device detection.
- Guarded firmware installation and runtime-return monitoring.
- Guided STM32 and Daisy Bootloader update flows.
- Serial diagnostic console, transcript export, and SQLite history.
- macOS development packaging and responsive visual design.
