# Thaw CLI

**⚠️ This project is still under development.**

Build tool for `Leptos`.

## Getting started

#### Install

`cargo install --git https://github.com/thaw-ui/thaw-cli --locked`

#### Configuration file

Add `Thaw.toml` in the project root directory. Please refer to the [Config structure](https://github.com/thaw-ui/thaw-cli/blob/main/crates/thaw_cli/src/config/mod.rs) for the type (Currently all configurations are optional).

#### Command

```shell
thaw serve csr
thaw serve ssr

thaw build csr
thaw build ssr
```

## Goals

The API is aligned with `Vite` (Rolldown) as much as possible.

Reuse existing libraries as much as possible to optimize the development experience. e.g.: `Subsecond`, `Manganis`.

## Resources

[Vite](https://github.com/vitejs/vite) - Native-ESM powered web dev build tool.

[Dioxus CLI](https://github.com/DioxusLabs/dioxus/tree/main/packages/cli) - Tooling to supercharge Dioxus projects.

[cargo-leptos](https://github.com/leptos-rs/cargo-leptos) - Build tool for Leptos.
