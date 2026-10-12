"""Check placement baking, real native capture export and corrupt-file rejection."""
import copy
import importlib.util
import json
import math
import pathlib
import sys
import unittest

spec = importlib.util.spec_from_file_location('instance_cache', pathlib.Path(__file__).with_name('export-instance-cache.py'))
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


def fixture():
    placement = {'position': [10, 20, 30], 'orientation_wxyz': [math.sqrt(.5), 0, 0, math.sqrt(.5)],
                 'scale': [2, 3, -4], 'color_components': [.25, .5, .75, 1]}
    batch = {'build_id': 7, 'builder': 'fixture', 'queued_submeshes': 2,
             'placement_schema_version': 1, 'placement_space': 'builder_input',
             'placement_color_order': 'renderer_native',
             'batch_origin': [12, -2, 7],
             'effective_material': 'rock_runtime',
             'omitted_submeshes': 0, 'omitted_placements': 0, 'invalid_placements': 0,
             'sources': [{'mesh': 'rock.mesh', 'material': 'rock', 'vertices_per_copy': 126,
                          'indices_per_copy': 126, 'copies': 2,
                          'placements': [placement, copy.deepcopy(placement)]}]}
    resource = {'description': {'kind': 'buffer', 'bind_flags': 1, 'usage': 0}, 'batch': batch}
    return {'schema_version': 3, 'session_id': 'fixture', 'resources': [resource, copy.deepcopy(resource)]}


class CacheTests(unittest.TestCase):
    def test_affine_rotation_nonuniform_mirrored_scale_and_color(self):
        values = cache.RECORD.unpack(cache.affine_record(fixture()['resources'][0]['batch']['sources'][0]['placements'][0]))
        # Independent geometric expectation: (1, 2, 3) scaled (2,6,-12),
        # quarter-turned about Z (-6,2,-12), translated (4,22,18).
        point = [1, 2, 3, 1]
        actual = [sum(a*b for a, b in zip(values[row*4:row*4+4], point)) for row in range(3)]
        for a, b in zip(actual, [4, 22, 18]):
            self.assertAlmostEqual(a, b, places=5)
        self.assertEqual(values[12:], (.25, .5, .75, 1))

    def test_chunks_deduplicate_vertex_streams_and_roundtrip(self):
        packed, metadata = cache.bake(fixture(), 'input-digest')
        decoded, payload = cache.read_cache(packed)
        self.assertEqual(decoded, metadata)
        self.assertEqual(len(payload), 2 * 64)
        self.assertEqual(len(metadata['chunks']), 1)
        self.assertEqual(metadata['chunks'][0]['sources'][0]['record_count'], 2)

    def test_conflicting_copies_and_incomplete_old_capture_rejected(self):
        report = fixture()
        report['resources'][1]['batch']['sources'][0]['material'] = 'changed'
        with self.assertRaisesRegex(ValueError, 'Conflicting'):
            cache.bake(report, '')
        for key in ('omitted_submeshes', 'omitted_placements', 'invalid_placements'):
            report = fixture()
            for resource in report['resources']:
                resource['batch'][key] = 1
            with self.assertRaisesRegex(ValueError, 'incomplete'):
                cache.bake(report, '')
        report = fixture()
        for resource in report['resources']:
            del resource['batch']['placement_schema_version']
        with self.assertRaisesRegex(ValueError, 'capture again'):
            cache.bake(report, '')

    def test_corrupt_truncated_and_out_of_range_records_rejected(self):
        packed, _ = cache.bake(fixture(), '')
        for data in (packed[:10], packed[:-1], packed + b'x', packed[:-1] + bytes([packed[-1] ^ 1])):
            with self.assertRaises(ValueError):
                cache.read_cache(data)
        _, version, size, count = cache.HEADER.unpack_from(packed)
        metadata = json.loads(packed[cache.HEADER.size:cache.HEADER.size + size])
        metadata['chunks'][0]['sources'][0]['first_record'] = 1
        encoded = json.dumps(metadata).encode()
        malformed = cache.HEADER.pack(cache.MAGIC, version, len(encoded), count) + encoded + packed[cache.HEADER.size + size:]
        with self.assertRaisesRegex(ValueError, 'range'):
            cache.read_cache(malformed)

    def test_nonfinite_and_nonunit_transform_rejected(self):
        placement = fixture()['resources'][0]['batch']['sources'][0]['placements'][0]
        placement['position'][0] = float('nan')
        with self.assertRaisesRegex(ValueError, 'Non-finite'):
            cache.affine_record(placement)
        placement['position'][0] = 0
        placement['orientation_wxyz'] = [0, 0, 0, 0]
        with self.assertRaisesRegex(ValueError, 'Non-unit'):
            cache.affine_record(placement)


if __name__ == '__main__':
    unittest.main()
