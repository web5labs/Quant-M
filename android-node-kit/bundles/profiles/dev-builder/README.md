# Quant-M Android Dev Builder Profile

Heavy offline profile for a device that needs to build Rust code locally.

This is not the default for old devices. Prefer `base-runtime` and deploy a prebuilt Quant-M binary.

## Includes

- `openssh`
- `git`
- `curl`
- `termux-tools`
- `termux-api`
- `rust` and Cargo
- LLVM/Clang dependencies pulled by Rust
- `rsync`
