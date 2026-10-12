"""Reconcile every private resident byte in a saved startup ownership capture.

This reads saved evidence only. Stack labels describe the initial commit path,
not the current objects in a reused heap or graphics driver allocation.
"""
import argparse
import bisect
import collections
import json
from pathlib import Path, PureWindowsPath


GRAPHICS = ('nvwgf2umx', 'd3d11.dll', 'dxgkrnl', 'dxgmms2', 'nvlddmkm')


def category(stack):
    text = ' '.join(stack).lower()
    if any(module in text for module in GRAPHICS):
        return 'Graphics allocation paths'
    if 'plugin_terrain' in text:
        return 'Terrain allocation paths'
    if 'ogremain' in text:
        return 'Ogre allocation paths, excluding graphics and terrain'
    if 'kenshi_x64' in text:
        return 'Kenshi allocation paths, excluding the above'
    return 'Other observed allocation paths'


def summarize(directory):
    ownership = json.loads((directory / 'ownership.json').read_text())
    snapshot = json.loads((directory / 'hub-residency.json').read_text())
    if ownership['process_creation_time'] != snapshot['process_creation_time']:
        raise ValueError('Process identity mismatch')
    totals = collections.Counter()
    modules = json.loads((directory / 'modules.json').read_text(encoding='utf-8-sig'))
    executable = next(row for row in modules if row['ModuleName'].lower() == 'kenshi_x64.exe')
    game_directory = PureWindowsPath(executable['FileName']).parent
    game_modules = {row['ModuleName'].lower(): row['ModuleName'] for row in modules
                    if PureWindowsPath(row['FileName']).is_relative_to(game_directory)}
    first_application = collections.Counter()
    for owner in ownership['owners']:
        name = category(owner['stack'])
        totals[name] += owner['private_resident_bytes']
        if name == 'Graphics allocation paths':
            nearest = name
        else:
            nearest = next((game_modules[frame.partition('!')[0].lower()]
                            for frame in owner['stack']
                            if frame.partition('!')[0].lower() in game_modules),
                           '[no game-directory frame]')
        first_application[nearest] += owner['private_resident_bytes']
    intervals = sorted((int(row['address'], 16),
                        int(row['address'], 16) + row['size'])
                       for row in ownership['live_ranges'])
    starts = [left for left, _ in intervals]
    for previous, following in zip(intervals, intervals[1:]):
        if previous[1] > following[0]:
            raise ValueError('Overlapping ownership ranges')
    unknown_files = collections.Counter()
    private_total = resident_total = 0
    for region in snapshot['regions']:
        for run in region['resident_runs']:
            resident_total += run['size']
            if not run['private']:
                continue
            private_total += run['size']
            left = int(run['address'], 16)
            right = left + run['size']
            cursor = left
            index = max(0, bisect.bisect_right(starts, left) - 1)
            matched = 0
            while index < len(intervals) and intervals[index][0] < right:
                a, b = intervals[index]
                matched += max(0, min(right, b) - max(cursor, a))
                cursor = max(cursor, b)
                index += 1
            missing = run['size'] - matched
            if missing:
                label = ('Private image pages without an observed allocation'
                         if region['kind'] == 'image'
                         else 'Private pages without an observed allocation')
                totals[label] += missing
                first_application[label] += missing
                unknown_files[(region['kind'], region.get('file') or '(anonymous)')] += missing
    unmatched = sum(unknown_files.values())
    if unmatched != ownership['unmatched_resident']['private_resident_bytes']:
        raise ValueError('Unmatched-byte reconciliation failed')
    if sum(totals.values()) != private_total:
        raise ValueError('Private resident ledger does not add up')
    if sum(first_application.values()) != private_total:
        raise ValueError('First application module ledger does not add up')
    if resident_total != ownership['resident_total_bytes']:
        raise ValueError('Total resident denominator differs')
    graphics_file = directory / 'graphics-resident-breakdown.json'
    graphics = json.loads(graphics_file.read_text()) if graphics_file.exists() else None
    if graphics is not None:
        graphics_total = sum(row['private_resident_bytes'] for row in graphics.values())
        if graphics_total != totals['Graphics allocation paths']:
            raise ValueError('Graphics subpaths do not reconcile with the ledger')
    return dict(
        pid=snapshot['pid'], process_creation_time=snapshot['process_creation_time'],
        scan_started_utc=snapshot['scan_started_utc'], scan_ended_utc=snapshot['time'],
        private_resident_bytes=private_total, total_resident_bytes=resident_total,
        shared_resident_bytes=resident_total - private_total,
        ledger=dict(totals),
        first_application_ledger=dict(first_application),
        first_application_semantics='Graphics paths first; otherwise select the nearest stack frame in a module loaded from the game installation directory. This avoids attributing other allocators to a deeper Ogre game-loop caller. Identifies initial commit context, not current object ownership.',
        unmatched_by_file=[dict(kind=kind, file=file, private_resident_bytes=size)
                           for (kind, file), size in unknown_files.most_common()],
        graphics_subpaths=graphics,
        semantics='Disjoint initial-commit stack categories, ordered graphics, terrain, Ogre, Kenshi, other. Images and other unmatched private pages are included explicitly. These are resident bytes, not logical graphics payload. Current object ownership inside reused backing allocations is unresolved.',
        graphics_predicate=list(GRAPHICS),
        exact_sum_verified=True,
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path)
    args = parser.parse_args()
    result = summarize(args.capture)
    (args.capture / 'ram-ledger.json').write_text(json.dumps(result, indent=2))
    for name, size in result['ledger'].items():
        print(f'{size / 1048576:12.6f} MiB  {name}')
    print(f"{result['private_resident_bytes'] / 1048576:12.6f} MiB  TOTAL PRIVATE")
    print(f"{result['shared_resident_bytes'] / 1048576:12.6f} MiB  SHARED (additional)")
    print(f"{result['total_resident_bytes'] / 1048576:12.6f} MiB  TOTAL WORKING SET")
