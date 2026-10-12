"""Compare copied resource snapshots from the same running capture session."""
import argparse
import collections
import json
import pathlib


def read(path):
    if path.is_dir():
        path = path / 'summary.json'
    return json.loads(path.read_text(encoding='utf-8'))


def payload(resources):
    return sum(r['estimated_payload_bytes'] or 0 for r in resources) / 2**20


def compare(before, after):
    if before.get('schema_version') not in (2, 3) or after.get('schema_version') not in (2, 3):
        raise ValueError('Both snapshots must use capture schema 2 or 3')
    if before['session_id'] != after['session_id']:
        raise ValueError('Snapshots must belong to the same capture session')
    if before['elapsed_us'] >= after['elapsed_us']:
        raise ValueError('The second snapshot must be newer')
    old = {r['id']: r for r in before['resources']}
    new = {r['id']: r for r in after['resources']}
    added = [new[i] for i in new.keys() - old.keys()]
    retired = [old[i] for i in old.keys() - new.keys()]
    survived = [new[i] for i in old.keys() & new.keys()]
    quiet = [r for r in survived if r['description']['usage'] == 3
             and r['activity'] == old[r['id']]['activity']]
    print('Logical resource payload only; this does not measure RAM or VRAM residency.')
    print(f"Window: {(after['elapsed_us'] - before['elapsed_us']) / 1e6:.1f} seconds")
    for label, resources in [('Added', added), ('Retired', retired), ('Survived', survived)]:
        print(f'{label}: {len(resources)} resources; {payload(resources):.2f} MiB')
    print(f'Live payload delta: {payload(new.values()) - payload(old.values()):+.2f} MiB')
    print(f'Surviving staging with no observed activity in this window: {len(quiet)} resources; {payload(quiet):.2f} MiB')
    print(after['activity_semantics'])
    for label, resources in [('Added', added), ('Retired', retired)]:
        groups = collections.Counter()
        for r in resources:
            name = r['ogre_load_name'] or r['debug_name']
            if not name:
                name = ' <- '.join(f for f in r['origin_frames'] if f.lower().startswith(('kenshi', 'ogremain', 'rendersystem')))
            groups[name or '<unnamed>'] += r['estimated_payload_bytes'] or 0
        print(f'\nLargest {label.lower()} name/stack groups:')
        for name, size in groups.most_common(10):
            print(f'  {size / 2**20:.2f} MiB | {name}')
    for report in (before, after):
        print('Coverage gaps:', ', '.join(f'{k}={report[k]}' for k in (
            'untracked_creations', 'attachment_failures', 'activity_tag_failures', 'dropped_events', 'io_errors')))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=pathlib.Path)
    parser.add_argument('after', type=pathlib.Path)
    args = parser.parse_args()
    try:
        compare(read(args.before), read(args.after))
    except ValueError as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()
