# aski-cc — The Aski Compiler

Written in aski. Compiled by aski-rs. Does the interesting work.

## Architecture

Two databases:

- **Surface DB** — what the programmer wrote. Full v0.9 syntax:
  destructure arms, matching methods, module headers, grammar rules.
  The aski codec reads/writes this. Semantic queries work on this.

- **Kernel DB** → **Kernel Aski** — the macro-expanded form.
  Simple: domains, structs, computed methods, explicit match, sequential ops.
  aski-rs reads this and emits Rust.

## Pipeline

```
.aski source → Aski Parser (aski) → Surface DB (CozoDB)
                                          │
                                   Macro Expansion (aski)
                                          │
                                          ▼
                                    Kernel Aski (text + rkyv)
                                          │
                                    aski-rs → Rust → rustc
```

## Current State (2026-04-02)

- Pure aski parser: ZERO FFI stubs
- Parses all 13 items from astro-aski chart.aski (7 domains, 3 structs, 3 impls)
- DataCarrying pattern binding: `(Parsed(@Toks))` extracts inner Tokens
- Actor model: observe borrows, transform moves
- Tail-recursive item counting via `continueItems` matching method
- Balanced delimiter skipping (nested parens, braces, brackets, composition)

## Build

aski-cc is compiled from aski source by aski-rs:

```
aski-rs compiles aski/*.aski → Rust → aski-cc binary
```

## Dependencies

- `aski-rs` (local path: ../aski-rs) — build + dev dependency
- The .aski source files in aski/ are the actual compiler

## VCS

Jujutsu (`jj`) mandatory. Git is storage backend only.
