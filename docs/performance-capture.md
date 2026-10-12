# Capture a Kenshi baseline

Use a save where the slowdown is easy to reproduce. Keep the same mods, plugin
toggles, graphics settings, camera position, and game speed for comparisons.
The script does not launch or modify the game. Captures stay under the ignored
`target/performance` directory.

1. Launch Kenshi and load the save. Let initial loading finish.
2. Open PowerShell as Administrator (Windows tracing requires elevation).
3. Run:

```powershell
Set-Location 'C:\Users\Jayson\Documents\projects\rekenshi-rust'
.\scripts\capture-performance.ps1 -Scenario town -Seconds 60 -Notes 'Save: ...; camera: ...; game speed: ...; settings/mods: ...'
```

4. Return to the game. For `town`, hold the camera steady in a busy scene. For
   `battle`, reproduce the same encounter. For `travel`, follow a repeatable route;
   that profile also records disk and file activity. Avoid changing settings
   during the recording. Exclude the initial window-switch interval in analysis.
5. Report the printed capture folder in chat. No upload is needed when working
   in this local workspace.

Each run produces `trace.etl`, `memory.csv`, and `capture.json`. Metadata records
the PID, executable hash, initially loaded module addresses, requested scenario,
and notes. It does not automatically inventory mods, settings, or plugin toggles;
record these in Notes. Modules loaded later are captured by ETW image events.
WPR records system-wide activity, including file paths when FileIO is enabled;
analysis should filter to the recorded Kenshi PID. Keep raw traces local.

The recorder uses a unique WPR instance and never cancels an existing recording.
Let it finish; forcibly closing its terminal can leave a recording active. The
instance name is saved in `capture.json`. A failed stop prints a recovery command.
`-WhatIf` previews the capture without requiring a running game or starting WPR.

## Analysis and comparison

Open the ETL in Windows Performance Analyzer. Check trace event loss, select the
steady gameplay interval, and filter CPU Usage (Sampled) to the captured PID.
Rank stacks by CPU weight and distinguish self time from inclusive time. Use CPU
Usage (Precise), disk activity and GPU activity to distinguish execution from
waiting. Missing private symbols may leave module-relative addresses; map those
to the fingerprinted binary for focused disassembly. Some stacks may be incomplete.

Memory samples contain cumulative process CPU seconds, private committed bytes,
resident working-set bytes, and thread counts. They do not identify allocations
or establish a memory leak. A 60-second recording is a CPU baseline, not a
long-session leak test. Investigate sustained growth with repeated equivalent
routes and targeted allocation tracing after the first baseline.

Repeat the chosen scenario at least three times before and after an optimization.
Separate first-visit loading from warmed caches. Check frame/present timing when
available in the GPU trace, CPU cost, and loading stalls; do not claim average FPS
or latency percentiles from process-memory samples. No benchmark result is claimed
until a real gameplay trace is analyzed.

See [the initial investigation](research/cpu-investigation.md) for candidate
boundaries and Ghidra/REA availability. WPR syntax was checked against local
`wpr -help` and [Microsoft's documentation](https://learn.microsoft.com/en-us/windows-hardware/test/wpt/wpr-command-line-options).
