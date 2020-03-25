# Changelog

This document is the changelog of [luminance-gl](https://crates.io/crates/luminance-gl).
You should consult it when upgrading to a new version, as it contains precious information on
breaking changes, minor additions and patch notes.

**If you’re experiencing weird type errors when upgrading to a new version**, it might be due to
how `cargo` resolve dependencies. `cargo update` is not enough, because all luminance crate use
[SemVer ranges](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) to stay
compatible with as many crates as possible. In that case, you want `cargo update --aggressive`.

<!-- vim-markdown-toc GFM -->

* [1.0](#10)
* [Pre 1.0](#pre-10)

<!-- vim-markdown-toc -->

# 1.0

> ?

# Pre 1.0

- The crate was available on https://crates.io with a different scope. If you were using it, please update to
  the latest [luminance](https://crates.io/crates/luminance) architecture.
