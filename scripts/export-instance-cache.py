"""Bake captured forest placements into indexed, 64-byte affine/color records.

This is an offline prototype artifact, not an installed game-renderer cache.
Coordinates stay in the captured builder input space. A build is a chunk here,
not a world zone: its world origin and native lifetime must be bound separately.
"""
import argparse
import hashlib
import json
import math
import pathlib
import struct

HEADER = struct.Struct('<8sIIQ')
RECORD = struct.Struct('<16f')
MAGIC = b'REKINST1'
MAX_RECORDS = 1_000_000
MAX_METADATA = 16 * 1024 * 1024


def affine_record(placement):
    """Pack a unit wxyz quaternion, column scale, translation and queue color."""
    position = placement['position']
    rotation = placement['orientation_wxyz']
    scale = placement['scale']
    color = placement['color_components']
    if tuple(map(len, (position, rotation, scale, color))) != (3, 4, 3, 4):
        raise ValueError('Malformed placement dimensions')
    if not all(math.isfinite(v) for a in (position, rotation, scale, color) for v in a):
        raise ValueError('Non-finite placement')
    if abs(sum(v * v for v in rotation) - 1) > 0.002:
        raise ValueError('Non-unit quaternion; cannot assume affine rotation semantics')
    w, x, y, z = rotation
    sx, sy, sz = scale
    rows = (
        (1 - 2 * (y*y + z*z)) * sx, 2 * (x*y - w*z) * sy,
        2 * (x*z + w*y) * sz, position[0],
        2 * (x*y + w*z) * sx, (1 - 2 * (x*x + z*z)) * sy,
        2 * (y*z - w*x) * sz, position[1],
        2 * (x*z - w*y) * sx, 2 * (y*z + w*x) * sy,
        (1 - 2 * (x*x + y*y)) * sz, position[2],
        *color,
    )
    try:
        packed = RECORD.pack(*rows)
    except (OverflowError, struct.error) as error:
        raise ValueError('Placement does not fit float32') from error
    if not all(math.isfinite(v) for v in RECORD.unpack(packed)):
        raise ValueError('Affine matrix overflow')
    return packed


def bake(report, source_digest):
    if report.get('schema_version') != 3:
        raise ValueError('Expected capture schema 3 with placement schema 1')
    batches = {}
    for resource in report['resources']:
        description = resource['description']
        batch = resource.get('batch')
        if not batch or description['kind'] != 'buffer' or not description['bind_flags'] & 1 or description['usage'] == 3:
            continue
        identity = batch['build_id']
        if identity in batches and batches[identity] != batch:
            raise ValueError(f'Conflicting records for build {identity}')
        batches[identity] = batch
    chunks, records = [], bytearray()
    for identity, batch in sorted(batches.items()):
        if batch.get('placement_schema_version') != 1 or batch.get('placement_space') != 'builder_input':
            raise ValueError(f'Build {identity}: no supported placement capture; capture again with updated host')
        if batch.get('placement_color_order') != 'renderer_native':
            raise ValueError(f'Build {identity}: unsupported queue color order')
        if any(batch.get(key, 0) for key in ('omitted_submeshes', 'omitted_placements', 'invalid_placements')):
            raise ValueError(f'Build {identity}: incomplete placement capture')
        origin = batch['batch_origin']
        if len(origin) != 3 or not all(math.isfinite(v) for v in origin):
            raise ValueError(f'Build {identity}: invalid batch origin')
        material = batch['effective_material']
        if not isinstance(material, str) or not material:
            raise ValueError(f'Build {identity}: no effective batch material')
        sources, count = [], 0
        for source in batch['sources']:
            placements = source['placements']
            if len(placements) != source['copies']:
                raise ValueError(f'Build {identity}: source placement count mismatch')
            offset = len(records) // RECORD.size
            if offset + len(placements) > MAX_RECORDS:
                raise ValueError('Instance cache exceeds record bound')
            for placement in placements:
                records.extend(affine_record(placement))
            count += len(placements)
            sources.append({
                'mesh': source['mesh'], 'material': source['material'],
                'vertices_per_copy': source['vertices_per_copy'],
                'indices_per_copy': source['indices_per_copy'],
                'first_record': offset, 'record_count': len(placements),
            })
        if count != batch['queued_submeshes']:
            raise ValueError(f'Build {identity}: batch placement count mismatch')
        chunks.append({'build_id': identity, 'builder': batch['builder'],
                       'batch_origin': origin, 'effective_material': material, 'sources': sources})
    if not chunks:
        raise ValueError('No live forest vertex buffers with placement data')
    metadata = {
        'schema_version': 1, 'session_id': report['session_id'],
        'source_summary_sha256': source_digest, 'coordinate_space': 'builder_input',
        'record_layout': 'little-endian float32: affine rows 3x4, then captured queue color 4',
        'material_semantics': 'Source material is the SubMesh asset name; effective_material is the live batch material name. Neither is a persistent content fingerprint.',
        'record_bytes': RECORD.size, 'record_count': len(records) // RECORD.size,
        'payload_sha256': hashlib.sha256(records).hexdigest(),
        'limitations': 'Prototype only. Build IDs are session-local, not persistent region IDs. '
                        'Ancestor/world transforms, game/mod/save compatibility, native lifetimes, shader and shadow '
                        'integration are not established by this file. Queue color interpretation follows '
                        'the native renderer. Geometry, collision and navigation are not stored.',
        'chunks': chunks,
    }
    encoded = json.dumps(metadata, separators=(',', ':'), allow_nan=False).encode('utf-8')
    if len(encoded) > MAX_METADATA:
        raise ValueError('Instance cache metadata exceeds bound')
    return HEADER.pack(MAGIC, 1, len(encoded), metadata['record_count']) + encoded + records, metadata


def read_cache(data):
    """Validate file bounds, checksum and every indexed source range before use."""
    if len(data) < HEADER.size:
        raise ValueError('Truncated cache header')
    magic, version, size, count = HEADER.unpack_from(data)
    if magic != MAGIC or version != 1 or size > MAX_METADATA or count > MAX_RECORDS:
        raise ValueError('Unsupported or oversized instance cache')
    start = HEADER.size + size
    if len(data) != start + count * RECORD.size:
        raise ValueError('Truncated cache or trailing data')
    metadata = json.loads(data[HEADER.size:start])
    payload = data[start:]
    if metadata['schema_version'] != 1 or metadata['coordinate_space'] != 'builder_input':
        raise ValueError('Unsupported cache metadata')
    if metadata['record_count'] != count or metadata['record_bytes'] != RECORD.size:
        raise ValueError('Cache metadata/header disagreement')
    if hashlib.sha256(payload).hexdigest() != metadata['payload_sha256']:
        raise ValueError('Cache payload checksum mismatch')
    cursor = 0
    for chunk in metadata['chunks']:
        origin = chunk['batch_origin']
        if len(origin) != 3 or not all(math.isfinite(v) for v in origin):
            raise ValueError('Invalid batch origin')
        for source in chunk['sources']:
            n = source['record_count']
            if not isinstance(n, int) or n < 0 or source['first_record'] != cursor or cursor + n > count:
                raise ValueError('Invalid instance range')
            cursor += n
    if cursor != count:
        raise ValueError('Unreferenced instance records')
    if not all(math.isfinite(v) for row in RECORD.iter_unpack(payload) for v in row):
        raise ValueError('Non-finite cached record')
    return metadata, payload


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=pathlib.Path)
    parser.add_argument('output', type=pathlib.Path)
    args = parser.parse_args()
    source = args.capture / 'summary.json' if args.capture.is_dir() else args.capture
    try:
        raw = source.read_bytes()
        data, metadata = bake(json.loads(raw), hashlib.sha256(raw).hexdigest())
        read_cache(data)
        # Exclusive creation keeps prior captures reviewable and prevents clobbering.
        with args.output.open('xb') as stream:
            stream.write(data)
    except (ValueError, KeyError, TypeError, OSError) as error:
        parser.error(str(error))
    print(f"Baked {metadata['record_count']} placements in {len(metadata['chunks'])} build chunks; "
          f"{metadata['record_count'] * RECORD.size:,} transform/color bytes; {len(data):,} file bytes.")
    print('Offline prototype; no game rendering or memory allocation changed.')


if __name__ == '__main__':
    main()
