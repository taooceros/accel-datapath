# Persistent agent coding requirements

This file records durable coding requirements distilled from the GSD milestone decision trail. Agents and contributors must read it before writing or modifying project code.

## CR-001: Single source of truth

The codebase MUST maintain one canonical IDXD stack:

- `idxd-sys` owns raw C/UAPI/FFI and bindgen-backed hardware ABI.
- `idxd-rust` owns the safe Rust API and higher-level lifecycle logic.

Duplicate FFI packages, handwritten shadow ABI structs, or parallel submission/completion implementations MUST NOT be introduced.

## CR-002: API shape must reflect real operation semantics

Public APIs MUST expose the real ownership and data-flow semantics of the operation.

For memmove-like APIs:

- source MUST be explicit;
- destination/output MUST be explicit;
- source-first ordering SHOULD be used;
- source-only request shapes MUST NOT be used.

## CR-003: Zero-copy/minimal-copy is a design constraint

APIs MUST NOT hide avoidable allocations, CPU copy-back paths, or double-copy behavior behind convenience methods.

Convenience APIs are allowed only when they clearly preserve the intended zero-copy/minimal-copy model or are explicitly marked as non-performance test/diagnostic helpers.

## CR-004: Low-level hardware truth must be generated or centralized

Hardware descriptor, completion, and ABI layouts MUST come from bindgen/kernel-derived definitions or a documented wrapper over them.

Handwritten hardware layout duplication is prohibited unless justified, isolated, and covered by layout tests.

## CR-005: Shared lifecycle, specialized ergonomics

Common hardware control flow — queue submission, completion observation, retry/classification, and lifecycle handling — SHOULD be shared internally.

Public APIs MAY remain accelerator- or operation-specific when that improves readability, for example:

- `IdxdSession<Dsa>::memmove(...)`
- `IdxdSession<Iax>::crc64(...)`

Shared internals MUST NOT force an awkward public abstraction.

## CR-006: Keep abstractions small and replaceable

New abstractions MUST be:

- small;
- easy to reason about;
- easy to replace;
- free of avoidable duplicate paths.

A limited first-version abstraction is acceptable. A broad, ugly, hard-to-narrow framework is not.

## CR-007: Use helper crates only when they reduce complexity

Macros, builders, and error frameworks such as `bon` and `snafu` SHOULD be used only where they improve readability, diagnostics, validation, or source-chain preservation.

They MUST NOT be added merely for consistency or style if plain Rust is clearer.

## CR-008: Match evidence to claims

Code claims MUST be backed by evidence of matching strength:

- host-free tests prove contracts only;
- verifier scripts prove artifact/schema/failure behavior;
- prepared-host runs prove live hardware behavior;
- release-profile benchmarks prove performance claims.

Hardware or performance claims MUST NOT rely only on host-free tests.

## CR-009: Preserve useful diagnostics without payload leakage

Failure paths MUST preserve enough structured information to identify setup, submission, completion, validation, lifecycle, or benchmark failures.

Diagnostics MUST NOT log or serialize payload bytes unless explicitly approved for a narrow diagnostic purpose.

## CR-010: Set API/proof standards before implementation

Before implementing a non-trivial hardware/API change, the intended API shape, ownership model, proof standard, and accepted non-goals MUST be explicit.

Implementation MUST NOT proceed on a merely workable path if it would entrench a weak API, duplicate boundary, or insufficient proof standard.

## CR-011: Verification must be necessary and claim-scoped

Agents MUST NOT run non-necessary verification.

Every verification command MUST be tied to a specific claim, risk, acceptance criterion, changed file, or explicitly requested gate. When verification is required, use the narrowest command that can falsify the claim.

Agents MUST NOT run broad test suites, long-running benchmarks, hardware-facing verifiers, or unrelated checks as ritual proof when a smaller targeted check is sufficient. Hardware or benchmark verification is required only when the current claim depends on hardware or performance evidence, or when the user explicitly asks for it.

Before running verification, the agent SHOULD know what failure would change the next action. If no plausible failure would affect the work, the verification is non-necessary and should not be run.