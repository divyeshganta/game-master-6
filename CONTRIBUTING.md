# Contributing to Game Master 6

Thank you for contributing to Game Master 6!

## Workflow
1. **Fork** the repository and clone your fork locally.
2. **Create a branch** for your change (`git checkout -b fix/my-change` or `feat/my-feature`).
3. **Make your edits** keeping the codebase zero-external-crate (pure `std` + Win32 FFI).
4. **Run tests & release build** on Windows before opening a PR:
   ```powershell
   cargo test
   cargo build --release
   ```
5. **Open a Pull Request** against `main` with a clear summary of what changed and why.

## Rules & Hygiene
- **Never commit** third-party runners (`victory.exe`, etc.), game binaries, archives, shader caches, logs, screenshots, or personal `.settings` / `config.json` files.
- Keep all defaults generic and free of personal filesystem paths.
- Ensure `cargo test` passes with zero failures.
