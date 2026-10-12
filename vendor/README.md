# Vendored dependencies

| Crate | Version | License | Provenance |
|---|---:|---|---|
| `winit` | `0.30.13` | Apache-2.0 | Exact published crates.io package; archive SHA-256 `a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d`. Windows 11 uses the OS-suggested DPI rectangle while Windows 10 retains the released adjustment; see [winit#4341](https://github.com/rust-windowing/winit/pull/4341) and `vendor/patches/winit/01-windows-11-dpi.md`. Wayland text input is retained across unrelated capability removal and destroyed with its seat; see [winit#4747](https://github.com/rust-windowing/winit/pull/4747) and `vendor/patches/winit/02-wayland-text-input.md`. |
