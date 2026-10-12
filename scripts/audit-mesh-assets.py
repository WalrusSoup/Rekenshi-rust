"""Read-only Ogre mesh inventory. Disk geometry payload is not runtime residency.

Uses Ogre's documented v1 geometry/submesh chunks. Some installed exporters have
inaccurate container lengths: containers follow recognized child IDs, while raw
buffer leaves are strictly bounded. Errors remain visible rather than guessed.
"""
import argparse
import collections
import csv
import json
import pathlib
import struct


class Mesh:
    def __init__(self, data):
        self.data = data
        self.endian = '<' if data[:2] == b'\x00\x10' else '>'
        if data[:2] not in (b'\x00\x10', b'\x10\x00'):
            raise ValueError('Not an Ogre mesh header')
        self.version, self.start = self.string(2)
        if self.version not in ('[MeshSerializer_v1.100]', '[MeshSerializer_v1.8]', '[MeshSerializer_v1.41]'):
            raise ValueError('Unsupported version ' + self.version)
        self.vertices = self.vertex_bytes = self.index_bytes = self.indices = 0
        self.triangles = self.submeshes = self.edge_bytes = 0
        self.materials = []
        self.strides = collections.Counter()
        self.embedded_lod = False
        self.length_mismatches = 0
        self.skeletal = False

    def unpack(self, fmt, offset):
        return struct.unpack_from(self.endian + fmt, self.data, offset)

    def string(self, offset):
        end = self.data.index(b'\n', offset)
        return self.data[offset:end].decode('utf-8', errors='replace'), end + 1

    def header(self, offset):
        tag, size = self.unpack('HI', offset)
        if size < 6 or offset + size > len(self.data):
            raise ValueError(f'Invalid chunk size at {offset}: {size}')
        return tag, offset + 6, offset + size

    def geometry(self, offset):
        tag, pos, end = self.header(offset)
        if tag != 0x5000:
            raise ValueError('Expected geometry')
        count, = self.unpack('I', pos)
        self.vertices += count
        pos += 4
        while pos < end:
            child, body, finish = self.header(pos)
            if finish > end:
                raise ValueError('Geometry child exceeds geometry')
            if child == 0x5200:
                _, stride = self.unpack('HH', body)
                data_tag, begin, data_end = self.header(body + 4)
                if data_tag != 0x5210 or data_end != finish or data_end - begin != count * stride:
                    raise ValueError('Vertex buffer size does not match count/stride')
                self.vertex_bytes += count * stride
                self.strides[stride] += count
            elif child != 0x5100:
                raise ValueError(f'Unsupported geometry chunk {child:#x}')
            pos = finish
        if pos != end:
            raise ValueError('Geometry boundary mismatch')
        return end

    def submesh(self, offset):
        _, pos, declared_end = self.header(offset)
        material, pos = self.string(pos)
        self.materials.append(material)
        shared, count, wide = self.unpack('?I?', pos)
        pos += 6
        self.submeshes += 1
        self.indices += count
        self.index_bytes += count * (4 if wide else 2)
        pos += count * (4 if wide else 2)
        if pos > len(self.data):
            raise ValueError('Index buffer exceeds file')
        if not shared:
            pos = self.geometry(pos)
        operation = 4  # Ogre's default OT_TRIANGLE_LIST.
        while pos + 6 <= len(self.data):
            child, body, end = self.header(pos)
            if child not in (0x4010, 0x4100, 0x4200):
                break
            if child == 0x4010:
                operation, = self.unpack('H', body)
            pos = end
        self.triangles += count // 3 if operation == 4 else max(0, count - 2) if operation in (5, 6) else 0
        self.length_mismatches += int(pos != declared_end)
        return pos

    def parse(self):
        tag, pos, declared_end = self.header(self.start)
        if tag != 0x3000:
            raise ValueError('Expected mesh container')
        self.skeletal = bool(self.data[pos])
        pos += 1
        while pos < len(self.data):
            child, _, end = self.header(pos)
            if child == 0x4000:
                pos = self.submesh(pos)
            elif child == 0x5000:
                pos = self.geometry(pos)
            else:
                self.embedded_lod |= child == 0x8000
                if child == 0xB000:
                    self.edge_bytes += end - pos
                pos = end
        self.length_mismatches += int(pos != declared_end)
        return dict(version=self.version, vertices=self.vertices, triangles=self.triangles,
                    submeshes=self.submeshes, vertex_payload_bytes=self.vertex_bytes,
                    index_payload_bytes=self.index_bytes, edge_chunk_bytes=self.edge_bytes,
                    embedded_lod=self.embedded_lod, skeletal=self.skeletal,
                    vertex_strides=dict(self.strides), materials=self.materials,
                    container_length_mismatches=self.length_mismatches)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root', type=pathlib.Path, help='data or mod asset directory')
    parser.add_argument('--output', required=True, type=pathlib.Path)
    args = parser.parse_args()
    records = []
    for path in sorted(args.root.rglob('*.mesh')):
        record = dict(path=str(path.relative_to(args.root)), disk_bytes=path.stat().st_size)
        try:
            record.update(Mesh(path.read_bytes()).parse())
        except (ValueError, IndexError, struct.error) as error:
            record['error'] = str(error)
        records.append(record)
    args.output.mkdir(parents=True, exist_ok=True)
    report = dict(root=str(args.root.resolve()), measurement='serialized base geometry; not runtime RAM/VRAM',
                  limits='loose files under this root only; excludes archives/workshop/other roots, textures, CPU copies, procedural/batched geometry and external LOD references; no embedded LOD chunk does not prove no distance LOD',
                  meshes=records)
    (args.output / 'meshes.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
    fields = ['path', 'disk_bytes', 'vertices', 'triangles', 'submeshes', 'vertex_payload_bytes', 'index_payload_bytes', 'edge_chunk_bytes', 'embedded_lod', 'skeletal', 'container_length_mismatches', 'error']
    with (args.output / 'meshes.csv').open('w', newline='', encoding='utf-8') as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, extrasaction='ignore')
        writer.writeheader()
        writer.writerows(records)
    valid = [r for r in records if 'error' not in r]
    print(f'Meshes: {len(records)}; parsed: {len(valid)}; errors: {len(records)-len(valid)}')
    print(f"Disk: {sum(r['disk_bytes'] for r in records)/2**20:.2f} MiB; parsed base vertex payload: {sum(r['vertex_payload_bytes'] for r in valid)/2**20:.2f} MiB")
    print(f"Parsed meshes without embedded LOD chunk: {sum(not r['embedded_lod'] for r in valid)} (external LOD not evaluated)")
    for r in sorted(valid, key=lambda r:r['vertex_payload_bytes'], reverse=True)[:20]:
        print(f"{r['vertex_payload_bytes']/2**20:6.2f} MiB vertices | {r['vertices']:7,d} vertices | {r['triangles']:7,d} triangles | embedded LOD={r['embedded_lod']} | {r['path']}")
    print('Reports:', args.output)


if __name__ == '__main__':
    main()
