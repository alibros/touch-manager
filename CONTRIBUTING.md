# Contributing to Touch Manager

Thank you for helping make Touch 2 firmware management safer and easier.

## Before opening a change

- Use an issue for substantial new behavior or hardware-writing changes.
- Keep the webview untrusted: addresses, alternate settings, commands, and validation
  policy belong in the Rust backend.
- Do not add firmware binaries unless their provenance and redistribution license are
  explicit.
- Never test an experimental flash path on a device without a verified recovery route and
  a known-good image.

## Local checks

```sh
npm ci
npm test
npm run build
(cd src-tauri && cargo fmt --all -- --check)
```

Tests must not invoke a hardware-writing command. Hardware verification should be manual,
opt-in, and documented separately from automated tests.

## Pull requests

Describe:

- The user-facing outcome.
- Safety implications and failure recovery.
- Tests performed, including whether real hardware was used.
- Screenshots for visible interface changes.
- Catalog provenance and license information for firmware metadata changes.

Keep changes focused and do not include generated `dist`, `target`, installer, or
`node_modules` content.
