# Agent guidelines

## Rust work

Before writing, refactoring, reviewing, or auditing Rust, read the relevant sections of [Microsoft's Pragmatic Rust Guidelines](docs/guidelines/microsoft-rust-guidelines.txt). Apply them to the code in scope and cite guideline IDs when reporting audit findings. For a workspace audit, evaluate every guideline's full text, including style and conditional rules. Record one status and supporting evidence per guideline, check the table against the upstream index, and state verification limits.

Use the local [Rust best-practice skill](.agents/skills/rust-best-practice/SKILL.md) for dependency changes and validation. The project's complete verification workflow is in [scripts/verify.ps1](scripts/verify.ps1).

Apply the upstream rules according to their stated scope. Account for this project's Windows x64 host/plugin ABI, native object ownership, and game-thread requirements. Document any necessary exception with its reason and supporting evidence; keep an existing deviation visible until it is resolved or explicitly accepted.

## Guideline snapshot

- Source: <https://microsoft.github.io/rust-guidelines/agents/all.txt>
- Downloaded: 2026-10-07.
- Local copy: `docs/guidelines/microsoft-rust-guidelines.txt` (unaltered upstream text, including copyright and MIT license notice).
- SHA-256: `E69533B0541A5D185AD9D8304F5EB5CF4B033135B7046F89C499B6126DCED3B6`.

When refreshing the snapshot, download from the source URL, preserve its license notice, and update the date and checksum here.
