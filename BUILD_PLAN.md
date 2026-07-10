# Touch Manager Build Plan

## Product objective

Touch Manager is a cross-platform desktop application for discovering, understanding,
installing, restoring, and diagnosing firmware on the Synthux Touch 2. The normal user
flow must not require a terminal or knowledge of STM32 memory addresses.

The product supports two update experiences:

1. **Compatible firmware:** Touch Manager asks the running firmware to reboot into DFU,
   installs the selected instrument, and confirms that the device returned.
2. **Legacy or unresponsive firmware:** Touch Manager displays the BOOT/RESET gesture,
   listens for DFU continuously, and continues automatically once the device appears.

The physical BOOT/RESET path remains the permanent recovery mechanism.

## Scope

### Version 1

- Curated official, community, and local firmware library.
- Firmware detail pages with author, version, description, controls, faceplate, source,
  trust level, and update-mode requirements.
- Import and validation of raw `.bin` files.
- Detection of runtime CDC, STM32 ROM DFU, and Daisy Bootloader DFU states.
- Safe support for internal, `BOOT_SRAM`, and `BOOT_QSPI` binaries.
- Guided DFU entry and semantic flash progress.
- Guarded, self-contained `dfu-util` backend with fixed target profiles.
- Runtime return detection and contextual handling of the known post-transfer
  `get_status` disconnect.
- USB serial diagnostics, searchable console, and exported support transcript.
- Install history and local firmware cache.
- Offline bundled catalog with a path to signed remote catalog updates.
- macOS first, followed by Windows and Linux packaging.

### Later

- Signed `.touchfw` packages and community submission pipeline.
- TouchLink firmware SDK for identity, structured diagnostics, crash reports, and
  buttonless DFU.
- Generic Touch 2 control-surface diagnostic firmware.
- Optional Daisy Bootloader managed mode.
- Optional custom multi-slot bootloader.
- Optional integration with OndaKit/Daisy Bloom exports.

### Explicitly out of scope for version 1

- Building arbitrary Arduino, libDaisy, Plugdata, or Oopsy source projects.
- Editing instruments or DSP graphs.
- Writing STM32 option bytes.
- Arbitrary memory addresses, mass erase, readout-protection changes, or production
  programming.
- Claiming to identify legacy runtime firmware without a handshake or flash readback.

## Architecture

### Desktop application

- **Shell:** Tauri 2.
- **Interface:** React 19, TypeScript, and Vite.
- **Hardware authority:** Rust commands and services. The webview cannot execute a shell,
  choose flash addresses, or bypass validation.
- **Persistence:** SQLite for install history, devices, catalog state, and diagnostics;
  firmware artifacts stored by SHA-256 in the application data directory.

### Rust modules

- `firmware`: reads binaries, hashes content, validates vector tables, and classifies
  execution layout.
- `catalog`: loads the bundled catalog, resolves local artifacts, and exposes typed
  firmware metadata.
- `device`: enumerates USB and serial devices and models runtime/DFU state.
- `flash`: resolves the bundled flashing engine, converts approved profiles into fixed
  `dfu-util` invocations, and interprets progress/results.
- `diagnostics`: opens CDC serial ports and emits log events to the interface.
- `history`: records attempted and completed installs in SQLite.

### Approved target profiles

| Profile | Transport | DFU alt | Upload address | Supported execution |
| --- | --- | ---: | ---: | --- |
| `touch2-stm-internal-v1` | STM32 ROM DfuSe | 0 | `0x08000000` | Internal flash |
| `touch2-daisy-sram-v1` | Daisy Bootloader DfuSe | 0 | `0x90040000` | SRAM |
| `touch2-daisy-qspi-v1` | Daisy Bootloader DfuSe | 0 | `0x90040000` | QSPI |

No catalog document or frontend request may supply a raw address or alternate setting.

## Binary validation

Before a file can reach the flash command, the backend must validate:

- Non-empty regular file and supported maximum size.
- SHA-256 against the package/catalog value when one exists.
- Plausible initial stack pointer.
- Thumb reset vector.
- Reset vector region matches the declared execution layout:
  - `0x080...` for internal flash.
  - `0x240...` for `BOOT_SRAM`.
  - `0x900...` for `BOOT_QSPI`.
- Selected target profile matches the classified layout.
- Only approved Touch 2 target profiles are used.
- Only DFU alt 0 is accessible.

Raw local binaries are displayed as untrusted until the user reviews the inferred target.

## Flash state machine

`idle -> validating -> awaiting_device -> requesting_update -> awaiting_dfu -> ready ->
erasing -> writing -> leaving -> awaiting_runtime -> succeeded | recovery_required | failed`

Rules:

- A flash action is armed for one firmware and expires.
- The destructive action requires an explicit confirmation in the interface.
- Multiple matching DFU devices are an error in version 1.
- Runtime and DFU serial numbers are not assumed to match.
- A completed transfer followed by a leave-time `get_status` disconnect is provisional,
  not immediate failure. Runtime re-enumeration decides the final result where possible.
- Runtime-USB-less packages explicitly declare that confirmation is unavailable.
- The complete transcript remains available even when the normal UI shows semantic steps.

## Catalog and trust model

The bundled catalog is a versioned JSON document. Production catalog updates will be
signed independently from application updates. Each release contains:

- Stable ID, name, semantic version, and channel.
- Author, source URL, license, and provenance.
- Trust tier: official, reviewed community, or local.
- Board compatibility and approved target profile.
- Binary URL, size, and SHA-256.
- Required bootloader and TouchLink versions.
- Expected runtime USB behavior.
- Description, tags, controls, faceplate, and release notes.
- Recovery and diagnostic notes.

Remote metadata can select only a compiled-in profile enum. It cannot introduce commands,
addresses, or shell arguments.

## TouchLink firmware protocol

TouchLink is a small versioned control protocol for maintained firmware. Initial commands:

- `HELLO`
- `DEVICE_INFO`
- `FIRMWARE_INFO`
- `ENTER_UPDATE_MODE`
- `START_DIAGNOSTICS`
- `STOP_DIAGNOSTICS`
- `GET_LAST_CRASH`

The update request uses a short-lived nonce and an acknowledgement before reset. Internal
firmware calls the existing STM boot reset helper. Daisy Bootloader applications request
the infinite bootloader timeout. Legacy firmware remains fully supported through the
guided physical recovery flow.

## Delivery phases

### Phase 0 — technical foundation

- Application scaffold and typed frontend/backend boundary.
- Binary analyzer and profile planner.
- Device enumeration.
- Guarded flash backend and transcript parser.
- Synthetic unit tests; no automated hardware writes.

Exit criteria: all 23 staged binaries classify correctly and unsafe profile combinations
are rejected.

### Phase 1 — macOS vertical slice

- Polished library and firmware detail experience.
- Local artifact resolution/import.
- Live device state and guided DFU modal.
- Flash confirmation, progress, runtime return, and history.
- Serial console and support export.

Exit criteria: representative internal, `BOOT_SRAM`, and `BOOT_QSPI` packages complete
the flow on a dedicated test Seed, with recovery tested after interruption.

### Phase 2 — catalog and trust

- `.touchfw` schema.
- Signed catalog verifier and content-addressed download cache.
- Official/community channels, release notes, control maps, and faceplates.
- Rollback and first-use backup flow.

### Phase 3 — cooperative firmware

- Shared DaisyDuino/libDaisy TouchLink implementations.
- Runtime identity and software-triggered DFU.
- Structured diagnostic telemetry and persisted crash information.
- Rebuilt manager-compatible versions of maintained instruments.

### Phase 4 — distribution

- Self-contained Apple Silicon DMG with a checksum-pinned flashing sidecar.
- Signed/notarized macOS DMG.
- Signed Windows installer and explicit Driver Doctor for WinUSB.
- Debian package with narrow udev rules and AppImage fallback.
- Signed application updates and CI release matrix.

### Phase 5 — optional managed mode

- Daisy Bootloader v6.4 integration and compatible rebuild pipeline.
- Power-loss and rollback testing.
- Multi-slot feasibility prototype only after the single-image path is stable.

## Test strategy

- Unit tests for binary headers, target mismatch, size limits, and hash mismatch.
- Golden classification tests using catalog metadata for every staged firmware.
- Flash transcript fixtures for success, ordinary failure, leave-time disconnect, access
  denial, missing driver, multiple devices, and unplug during write.
- Mocked device-state transitions for runtime -> DFU -> runtime.
- Rust tests never invoke a hardware-writing command.
- Frontend component and workflow tests.
- Visual QA on macOS WebKit and Windows WebView2 dimensions.
- Hardware matrix: internal firmware, SRAM firmware, QSPI firmware, legacy USB-less
  firmware, cooperative firmware, and interrupted transfer recovery.

## Release gates

- No option-byte or arbitrary-address path exists in a production build.
- Every bundled artifact has provenance, size, hash, and target metadata.
- App and catalog update signatures are independently verified.
- Windows driver changes are explicit and reversible.
- Linux never requires running the full application as root.
- Physical BOOT/RESET recovery is always reachable from the interface.
- All seven existing repositories remain independent and unmodified.
