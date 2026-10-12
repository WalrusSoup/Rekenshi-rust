# Hub runtime instancing: memory result

Measured 2026-10-08. Installed host SHA-256:
`DF0B8129D423F2637035395DADECB4E2E249F139380E694ACF3E1B0B44168FD6`.

## Result

Runtime instancing reduces measured geometry payload. This run does **not**
demonstrate lower resident system RAM. Do not describe logical D3D resource
payload savings as physical RAM savings.

Both snapshots contain 428 scenery batches and 17,909 submitted submeshes.
The replacement is active on 112 batches / 1,080 placements. The remaining
batches use the original builder. Large remaining batches include several
different cactus or skeleton mesh prototypes within one batch; the current
replacement accepts a single exact source SubMesh.

| Measurement | Before | Instancing run |
| --- | ---: | ---: |
| Scenery vertex payload | 856.44 MiB | 694.76 MiB |
| Scenery index payload | 45.81 MiB | 38.85 MiB |
| Total process working set, initial snapshots | 2390.00 MiB | 2415.08 MiB |

351 queues match exactly when comparing builder, origin, source mesh geometry
counts, and every copied placement value. Of these, 90 use the replacement.
Their original expanded geometry totals 150,244,182 bytes; replacement transform
streams total 44,160 bytes. That is 143.24 MiB less geometry payload for those
matched queues. Prototype buffers already existed in the baseline and remain
shared. Shader constant buffers and metadata are outside this geometry total.
The other 77 queues do not match exactly; aggregate scenery savings of 168.63
MiB therefore have a weaker scene-equivalence guarantee.

An elevated read-only Windows performance-counter sample for PID 70160 found:

- Private working set: 2,443,128,832 bytes (2330.0 MiB).
- Total working set: 2,555,080,704 bytes (2436.7 MiB).
- Private committed bytes: 6,519,721,984 bytes (6217.7 MiB).
- Discrete GPU dedicated usage: 3,534,626,816 bytes (3370.9 MiB).
- Discrete GPU shared usage: 12,505,088 bytes (11.9 MiB).

These counters were sampled later than the copied graphics snapshot; they are
not an atomic sample and should not be added together. The old snapshot lacks
private-working-set and GPU counters. No paired GPU residency result is claimed.

## Resident page scan

A subsequent read-only `VirtualQueryEx` / `QueryWorkingSetEx` scan found:

| Virtual region type | Committed | Resident |
| --- | ---: | ---: |
| Private | 6162.9 MiB | 2300.4 MiB |
| Mapped | 580.7 MiB | 24.7 MiB |
| Image | 363.8 MiB | 95.6 MiB |

The scan did not read target page contents, force pages resident, trim the
working set, suspend the game, or modify game memory. Enumeration is non-atomic.
Region type identifies mapping semantics, **not the allocating subsystem**.
The private regions could contain game heaps, mesh copies, driver allocations,
or other data. This scan alone cannot assign their ownership.

## Confounders and next experiment

Texture payload differs by about 233.52 MiB across runs. Most of the difference
is named blood/splash/flame textures (eleven names each differ by 22,369,620
bytes). This is not an instancing geometry result. Texture load/lifetime
variation prevents attributing the entire private-commit difference to this
change.

The prior heap/VirtualAlloc trace identified large graphics allocation paths,
but recorded commit rather than resident pages and came from another process.
To target the user's RAM figure, capture allocation stacks from process startup
and correlate surviving allocation ranges with page residency in that same run.
Rank **resident bytes by allocating stack** before selecting the next RAM
optimization. Mixed-source scenery instancing remains a geometry opportunity;
it has not yet been shown to address the dominant resident RAM category.

## Local evidence

- Baseline pointer: `target/current-hub-placement-capture.txt`.
- Replacement pointer: `target/current-hub-runtime-instancing-capture.txt`.
- Replacement snapshot directory contains `comparison.json`,
  `paired-comparison.json`, `residency.json`, `summary.json`, and `process.json`.
- Diagnostic helpers: `target/compare-hub-instancing.py` and
  `target/resident-memory-snapshot.py`; these are investigation artifacts.
- Previous allocation trace: `target/memory-investigation/heap-20261008-145038`.

Windows counter interpretation follows Microsoft's
[process working-set documentation](https://learn.microsoft.com/en-us/windows/win32/procthread/process-working-set),
[GPU counter explanation](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/),
and [working-set page flags](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-psapi_working_set_ex_block).
