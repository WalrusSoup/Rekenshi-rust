"""Summarize a copied D3D11 resource inventory without treating it as RAM usage."""
import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path, help='capture directory or summary.json')
    parser.add_argument('--top', type=int, default=20)
    parser.add_argument('--mesh-audit', type=pathlib.Path, help='meshes.json from audit-mesh-assets.py; ambiguous matches remain listed')
    args = parser.parse_args()
    path = args.capture / 'summary.json' if args.capture.is_dir() else args.capture
    report = json.loads(path.read_text(encoding='utf-8'))
    if report.get('schema_version') not in (1, 2, 3):
        parser.error('Unsupported capture schema')
    resources = report['resources']
    print(report['measurement'])
    print(f"Created {report['created']}; retired {report['destroyed']}; live {report['live_count']}; final {report['final']}")
    print(f"Known live payload: {report['known_live_payload_bytes'] / 2**20:.2f} MiB; peak {report['peak_known_payload_bytes'] / 2**20:.2f} MiB; unknown-size live {report['unknown_size_live_count']}")
    print('Coverage gaps:', ', '.join(f'{key}={report[key]}' for key in ['untracked_creations', 'attachment_failures', 'dropped_events', 'io_errors']))
    kinds = collections.Counter()
    names = collections.Counter()
    shapes = collections.Counter()
    for resource in resources:
        description = resource['description']
        size = resource['estimated_payload_bytes'] or 0
        kinds[description['kind']] += size
        name = resource['ogre_load_name'] or resource['debug_name'] or '<unnamed>'
        names[name] += size
        shapes[tuple(sorted(description.items()))] += 1
    print('\nKnown payload by kind:')
    for kind, size in kinds.most_common():
        print(f'  {kind}: {size / 2**20:.2f} MiB')
    print('\nLargest live resources (DXGI formats and flags are numeric):')
    ordered = sorted(resources, key=lambda r: r['estimated_payload_bytes'] or 0, reverse=True)
    for resource in ordered[:max(0, args.top)]:
        d = resource['description']
        size = '?' if resource['estimated_payload_bytes'] is None else f"{resource['estimated_payload_bytes'] / 2**20:.2f} MiB"
        dimensions = f"{d['buffer_bytes']} bytes" if d['kind'] == 'buffer' else f"{d['width']}x{d['height']}x{d['depth']} array={d['array_size']} mips={d['mip_levels']} samples={d['samples']} format={d['format']}"
        name = resource['ogre_load_name'] or resource['debug_name'] or '<unnamed>'
        print(f"  #{resource['id']} {size} {d['kind']} {dimensions} usage={d['usage']} bind=0x{d['bind_flags']:x} CPU=0x{d['cpu_access_flags']:x} | {name}")
    print('\nLargest enclosing Ogre load/debug-name groups (correlation, not exclusive ownership):')
    for name, size in names.most_common(max(0, args.top)):
        print(f'  {size / 2**20:.2f} MiB | {name}')
    repeated = sum(count - 1 for count in shapes.values() if count > 1)
    print(f'\n{repeated} additional resources share descriptors with another live resource. This does not establish duplicate content.')
    print('Resource payload excludes driver overhead, CPU copies and residency. Heap/VirtualAlloc tracing remains a separate measurement.')
    stacks = collections.Counter()
    staging = [r for r in resources if r['description']['usage'] == 3]
    for resource in resources:
        frames = resource.get('origin_frames', [])
        # Keep game/renderer frames for grouping rather than this diagnostic DLL.
        origin = ' <- '.join(f for f in frames if f.lower().startswith(('kenshi', 'rendersystem', 'ogremain')))
        stacks[origin or '<no game/renderer frame>'] += resource['estimated_payload_bytes'] or 0
    if report.get('schema_version') in (2, 3):
        print('\nLargest creation-stack groups:')
        for stack, size in stacks.most_common(max(0, args.top)):
            print(f'  {size / 2**20:.2f} MiB | {stack}')
        untouched = [r for r in staging if r['activity']['last_activity_us'] is None]
        print(f"\nStaging: {len(staging)} live; {len(untouched)} have no observed activity ({sum(r['estimated_payload_bytes'] or 0 for r in untouched) / 2**20:.2f} MiB)")
        print(report['activity_semantics'])
        print(f"Activity tag failures: {report['activity_tag_failures']}")
    if report.get('schema_version') == 3:
        print('\nForest batch provenance:')
        print('Unavailable:', report.get('batch_attribution_unavailable') or 'no')
        print('Snapshot failures (process total):', report.get('batch_snapshot_failures_process_total', 0))
        geometry = [r for r in ordered if r.get('batch') and r['description']['kind'] == 'buffer'
                    and r['description']['bind_flags'] & 1 and r['description']['usage'] != 3]
        totals = collections.Counter()
        submissions = collections.Counter()
        seen = set()
        unweighted = 0
        assets = []
        if args.mesh_audit:
            assets = json.loads(args.mesh_audit.read_text(encoding='utf-8'))['meshes']
        def matches(name):
            normalized = name.replace('\\', '/').casefold()
            exact = [a['path'] for a in assets if a['path'].replace('\\', '/').casefold() == normalized]
            return exact or [a['path'] for a in assets if a['path'].replace('\\', '/').casefold().endswith('/' + normalized)
                             or a['path'].replace('\\', '/').rsplit('/', 1)[-1].casefold() == normalized.rsplit('/', 1)[-1]]
        for resource in geometry:
            batch = resource['batch']
            sources = batch['sources']
            vertices = sum(s['vertices_per_copy'] * s['copies'] for s in sources)
            expanded = batch.get('geometry_representation', 'expanded_vertices') == 'expanded_vertices'
            weightable = expanded and vertices > 0 and vertices == batch['output_vertices'] and not batch['omitted_submeshes']
            if not weightable:
                unweighted += resource['estimated_payload_bytes'] or 0
            for source in sources:
                key = (source['mesh'], source['material'])
                if weightable:
                    totals[key] += (resource['estimated_payload_bytes'] or 0) * source['vertices_per_copy'] * source['copies'] / vertices
                identity = (batch['build_id'], key, source['vertices_per_copy'], source['indices_per_copy'])
                # Multiple vertex streams repeat a queue; count that queue once.
                if identity not in seen:
                    submissions[key] += sum(s['copies'] for s in sources if (s['mesh'], s['material'], s['vertices_per_copy'], s['indices_per_copy']) == (*key, source['vertices_per_copy'], source['indices_per_copy']))
                    seen.add(identity)
        print(f"Attributed vertex buffers: {len(geometry)}, {sum(r['estimated_payload_bytes'] or 0 for r in geometry) / 2**20:.2f} MiB")
        builds = {r['batch']['build_id']: r['batch'] for r in geometry}
        instanced = [b for b in builds.values() if b.get('geometry_representation') == 'shared_mesh_instances']
        if instanced:
            print(f'Runtime shared-mesh builds: {len(instanced)}; expanded input counts are not allocated vertex counts.')
        placements = [b for b in builds.values() if b.get('placement_schema_version') == 1]
        if placements:
            captured = sum(len(s['placements']) for b in placements for s in b['sources'])
            omitted = sum(b['omitted_placements'] for b in placements)
            invalid = sum(b['invalid_placements'] for b in placements)
            print(f'Placement capture: {captured} records in {len(placements)} builds; omitted={omitted}, invalid={invalid}')
            print(f'Baked affine/color payload would be {captured * 64 / 2**20:.3f} MiB, excluding geometry, metadata and renderer overhead. Builder input coordinates; this is not an installed optimization.')
        print(f'Cannot apportion by vertices: {unweighted / 2**20:.2f} MiB')
        for (mesh, material), size in totals.most_common(max(0, args.top)):
            print(f'  {size / 2**20:.2f} MiB estimated vertex share | {submissions[(mesh, material)]} submesh submissions | {mesh} | {material}')
            if args.mesh_audit:
                candidates = matches(mesh)
                print('    Source candidates:', ' ; '.join(candidates) if candidates else '<not found in audited loose files>')
        print('\nLargest forest vertex buffers and input meshes:')
        for resource in geometry[:max(0, args.top)]:
            batch = resource['batch']
            print(f"  #{resource['id']} {(resource['estimated_payload_bytes'] or 0) / 2**20:.2f} MiB | {batch['builder']} build {batch['build_id']} | {batch['queued_submeshes']} submitted submeshes; {batch['output_vertices']} vertices; omitted {batch['omitted_submeshes']}")
            for source in sorted(batch['sources'], key=lambda s: s['vertices_per_copy'] * s['copies'], reverse=True)[:max(0, args.top)]:
                print(f"    {source['copies']} copies x {source['vertices_per_copy']} vertices | {source['mesh']} | {source['material']}")
        print(report['batch_semantics'])


if __name__ == '__main__':
    main()
