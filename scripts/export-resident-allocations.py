"""Join observed VirtualAlloc lifetime stacks to a same-process resident snapshot.

Input CSV is xperf -a dumper -stacktimeshifting output, filtered to memory and
stack providers. Neither this exporter nor the snapshot forces pages resident.
This measures resident bytes in VirtualAlloc backing ranges, not heap objects.
"""
from __future__ import annotations

import argparse
import bisect
import collections
import csv
import datetime as dt
import json
import pathlib
import re


MEMORY_PROVIDER = '{3d6fa8d3-fe05-11d0-9dda-00c04fd7ba7c}'
STACK_PROVIDER = '{def2fe46-7bd6-4b80-bd94-f57fe20d0ce3}'


def number(value):
    return int(value, 0) if isinstance(value, str) else value


class Intervals:
    """Disjoint committed half-open ranges, preserving first commit ownership."""

    def __init__(self):
        self.rows = []
        self.starts = []

    def overlap(self, start, end):
        index = max(0, bisect.bisect_right(self.starts, start) - 1)
        while index < len(self.rows):
            row = self.rows[index]
            if row[0] >= end:
                break
            if row[1] > start:
                yield row
            index += 1

    def commit(self, start, end, owner):
        cursor = start
        added = []
        for left, right, _ in self.overlap(start, end):
            if cursor < left:
                added.append((cursor, min(left, end), owner))
            cursor = max(cursor, right)
        if cursor < end:
            added.append((cursor, end, owner))
        for row in added:
            index = bisect.bisect_left(self.starts, row[0])
            self.rows.insert(index, row)
            self.starts.insert(index, row[0])

    def free(self, start, end):
        index = max(0, bisect.bisect_right(self.starts, start) - 1)
        finish = bisect.bisect_left(self.starts, end)
        replacements = []
        for left, right, owner in self.rows[index:finish]:
            if right <= start:
                replacements.append((left, right, owner))
                continue
            if left < start:
                replacements.append((left, start, owner))
            if right > end:
                replacements.append((end, right, owner))
        self.rows[index:finish] = replacements
        self.starts[index:finish] = [row[0] for row in replacements]


def export(csv_path, snapshot, cutoff_us, creation_time):
    if int(snapshot['process_creation_time']) != creation_time:
        raise ValueError('Process creation time mismatch; addresses from another process cannot be joined')
    pid = int(snapshot['pid'])
    intervals = Intervals()
    owners = []
    owner_ids = {}
    counters = collections.Counter()
    pending = None

    def flush():
        nonlocal pending
        if pending is None:
            return
        event, timestamp, start, end, flags, frames = pending
        pending = None
        counters[event] += 1
        if end <= start:
            counters['invalid_or_empty_ranges'] += 1
            return
        if event == 'VirtualFree':
            intervals.free(start, end)
            return
        # Reserve-only ranges do not represent committed or resident pages.
        if 'COMMIT' not in flags.upper():
            counters['reserve_only'] += 1
            return
        stack = tuple(frames or ())
        # Rundown describes preexisting mappings without original allocation stacks.
        if event != 'VirtualAlloc':
            stack = ()
        key = stack
        if key not in owner_ids:
            owner_ids[key] = len(owners)
            owners.append(dict(stack=list(stack), committed_bytes=0,
                               resident_bytes=0, private_resident_bytes=0))
        intervals.commit(start, end, owner_ids[key])

    wanted_events = ('VirtualAlloc', 'VirtualFree', 'VirtualAllocDCStart', 'VirtualAllocDCEnd')
    process_marker = f'({pid})'
    # Dumps can contain gigabytes of other processes' stack rows. Inspect only
    # their event name before invoking the considerably more expensive CSV parser.
    with pathlib.Path(csv_path).open(encoding='utf-8-sig', errors='replace', newline='', buffering=1024*1024) as stream:
        for line in stream:
            name = line.lstrip().partition(',')[0].rstrip()
            if name == 'Stack':
                if pending is None:
                    continue
                timestamp_field = line.split(',', 2)[1].strip()
                if not timestamp_field.isdigit() or int(timestamp_field) != pending[1]:
                    flush()
                    continue
            elif name in wanted_events:
                flush()
                if process_marker not in line:
                    continue
            else:
                if pending is not None:
                    flush()
                continue
            fields = [field.strip() for field in next(csv.reader([line], skipinitialspace=True))]
            if name == 'Stack' and pending is not None:
                if len(fields) >= 6 and fields[1].isdigit() and int(fields[1]) == pending[1]:
                    # xperf timeshifting combines fragments into contiguous numbered
                    # frames. A second first frame is ambiguous; keep its range but
                    # refuse to attribute those bytes to either stack.
                    if fields[3] == '1' and pending[5]:
                        counters['ambiguous_stack_groups'] += 1
                        pending[5] = None
                    elif pending[5] is not None:
                        pending[5].append(fields[5])
                continue
            flush()
            if name not in wanted_events:
                continue
            if len(fields) < 6 or not fields[1].isdigit():
                continue
            event_pid = re.search(r'\((\d+)\)\s*$', fields[2])
            if event_pid is None or int(event_pid[1]) != pid:
                continue
            timestamp = int(fields[1])
            if timestamp > cutoff_us:
                continue
            pending = [name, timestamp, int(fields[3], 16), int(fields[4], 16), fields[5], []]
    flush()
    for start, end, owner in intervals.rows:
        owners[owner]['committed_bytes'] += end - start

    unknown = collections.Counter()
    for region in snapshot['regions']:
        if 'resident_runs' not in region:
            raise ValueError('Snapshot must include resident_runs; aggregate region sizes cannot be divided among allocation owners')
        for run in region['resident_runs']:
            start = number(run['address'])
            end = start + int(run['size'])
            covered = 0
            for left, right, owner in intervals.overlap(start, end):
                size = min(end, right) - max(start, left)
                covered += size
                owners[owner]['resident_bytes'] += size
                if run['private']:
                    owners[owner]['private_resident_bytes'] += size
            missing = end - start - covered
            unknown['resident_bytes'] += missing
            if run['private']:
                unknown['private_resident_bytes'] += missing
    stack_by_id = {value: key for key, value in owner_ids.items()}
    owners.sort(key=lambda row: row['private_resident_bytes'], reverse=True)
    observed = sum(row['resident_bytes'] for row in owners)
    total = observed + unknown['resident_bytes']
    return dict(pid=pid, process_creation_time=creation_time,
                snapshot_time=snapshot['time'], cutoff_us=cutoff_us,
                semantics='Resident pages intersected with live observed VirtualAlloc COMMIT ranges. First commit stack retained until decommit/release. Heap backing ownership is not object ownership. Missing pre-trace allocations and image/mapped pages remain unknown. Non-atomic snapshot and ETW collection can race.',
                events=dict(counters), resident_total_bytes=total,
                resident_with_observed_range_bytes=observed,
                resident_with_observed_stack_bytes=sum(row['resident_bytes'] for row in owners if row['stack']),
                unmatched_resident=dict(unknown), owners=owners,
                live_ranges=[dict(address=hex(start), size=end-start,
                                  stack=list(stack_by_id[owner]))
                             for start, end, owner in intervals.rows])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('csv', type=pathlib.Path)
    parser.add_argument('snapshot', type=pathlib.Path)
    parser.add_argument('output', type=pathlib.Path)
    parser.add_argument('--process-creation-time', type=int, required=True,
                        help='FILETIME from capture metadata, must match snapshot (not copied from a different run)')
    stop = parser.add_mutually_exclusive_group(required=True)
    stop.add_argument('--cutoff-us', type=int, help='Snapshot time relative to ETL start, in microseconds')
    stop.add_argument('--trace-start-utc', help='UTC Start time from xperf tracestats, ISO8601')
    args = parser.parse_args()
    snapshot = json.loads(args.snapshot.read_text(encoding='utf-8-sig'))
    cutoff = args.cutoff_us
    if cutoff is None:
        # FILETIME-derived trace timestamps can have seven fractional digits;
        # Python versions before 3.11 accept at most six (submicrosecond loss).
        start_iso = re.sub(r'(\.\d{6})\d+', r'\1', args.trace_start_utc)
        start = dt.datetime.fromisoformat(start_iso.replace('Z', '+00:00'))
        end = dt.datetime.fromisoformat(snapshot['time'].replace('Z', '+00:00'))
        if start.tzinfo is None or end.tzinfo is None:
            parser.error('Trace and snapshot timestamps must specify UTC offsets')
        cutoff = int((end - start).total_seconds() * 1_000_000)
    if cutoff < 0:
        parser.error('Snapshot precedes trace start')
    report = export(args.csv, snapshot, cutoff, args.process_creation_time)
    args.output.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({key: value for key, value in report.items() if key not in ('owners', 'live_ranges')}, indent=2))
    for owner in report['owners'][:12]:
        print(f"{owner['private_resident_bytes']/1048576:.2f} MiB private resident; {owner['committed_bytes']/1048576:.2f} MiB observed commit; " + ' > '.join(owner['stack'][-8:]))


if __name__ == '__main__':
    main()
