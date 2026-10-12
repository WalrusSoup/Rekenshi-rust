"""Read-only page residency snapshot; never reads, faults in, or trims target pages."""
import argparse
import collections
import ctypes as c
import ctypes.wintypes as w
import datetime
import json
import pathlib
import time

parser = argparse.ArgumentParser()
parser.add_argument('pid', type=int)
parser.add_argument('output', type=pathlib.Path)
parser.add_argument('--top', type=int, default=0, help='Print the largest resident regions')
args = parser.parse_args()
k = c.WinDLL('kernel32', use_last_error=True)
p = c.WinDLL('psapi', use_last_error=True)

class MBI(c.Structure):
    _fields_ = [('base', c.c_void_p), ('allocation', c.c_void_p),
                ('allocation_protect', w.DWORD), ('partition', w.WORD),
                ('size', c.c_size_t), ('state', w.DWORD),
                ('protect', w.DWORD), ('kind', w.DWORD)]

class Page(c.Structure):
    _fields_ = [('address', c.c_void_p), ('flags', c.c_size_t)]

k.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
k.OpenProcess.restype = w.HANDLE
k.CloseHandle.argtypes = [w.HANDLE]
k.GetProcessTimes.argtypes = [w.HANDLE, c.POINTER(w.FILETIME), c.POINTER(w.FILETIME), c.POINTER(w.FILETIME), c.POINTER(w.FILETIME)]
k.GetProcessTimes.restype = w.BOOL
k.VirtualQueryEx.argtypes = [w.HANDLE, c.c_void_p, c.POINTER(MBI), c.c_size_t]
k.VirtualQueryEx.restype = c.c_size_t
p.GetMappedFileNameW.argtypes = [w.HANDLE, c.c_void_p, w.LPWSTR, w.DWORD]
p.GetMappedFileNameW.restype = w.DWORD
p.QueryWorkingSetEx.argtypes = [w.HANDLE, c.c_void_p, w.DWORD]
p.QueryWorkingSetEx.restype = w.BOOL
handle = k.OpenProcess(0x410, False, args.pid)
if not handle:
    raise c.WinError(c.get_last_error())
regions = []
totals = collections.defaultdict(collections.Counter)
address = 0
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
creation, exit_time, kernel_time, user_time = (w.FILETIME() for _ in range(4))
if not k.GetProcessTimes(handle, c.byref(creation), c.byref(exit_time), c.byref(kernel_time), c.byref(user_time)):
    k.CloseHandle(handle)
    raise c.WinError(c.get_last_error())
creation_ticks = (creation.dwHighDateTime << 32) | creation.dwLowDateTime
process_start_utc = datetime.datetime.fromtimestamp((creation_ticks - 116444736000000000) / 10_000_000, datetime.timezone.utc).isoformat()
monotonic_start = time.perf_counter()
try:
    while True:
        mbi = MBI()
        if not k.VirtualQueryEx(handle, address, c.byref(mbi), c.sizeof(mbi)):
            error = c.get_last_error()
            if error != 87:
                raise c.WinError(error)
            break
        if not mbi.size or (mbi.base or 0) + mbi.size <= address:
            raise RuntimeError('Non-progressing region enumeration')
        address = (mbi.base or 0) + mbi.size
        if mbi.state != 0x1000:
            continue
        kind = {0x20000: 'private', 0x40000: 'mapped', 0x1000000: 'image'}.get(mbi.kind, hex(mbi.kind))
        name = ''
        if kind in ('mapped', 'image'):
            buffer = c.create_unicode_buffer(32768)
            if p.GetMappedFileNameW(handle, mbi.base, buffer, len(buffer)):
                name = buffer.value
        resident = private_resident = 0
        runs = []
        for start in range(mbi.base or 0, address, 4096 * 16384):
            count = min((address - start) // 4096, 16384)
            pages = (Page * count)()
            for index in range(count):
                pages[index].address = start + index * 4096
            if not p.QueryWorkingSetEx(handle, pages, c.sizeof(pages)):
                raise c.WinError(c.get_last_error())
            for page in pages:
                if page.flags & 1:
                    resident += 4096
                    private = not bool(page.flags & (1 << 15))
                    if private:
                        private_resident += 4096
                    location = page.address
                    if runs and runs[-1]['end'] == location and runs[-1]['private'] == private:
                        runs[-1]['end'] += 4096
                    else:
                        runs.append(dict(start=location, end=location + 4096, private=private))
        row = dict(address=hex(mbi.base or 0), allocation=hex(mbi.allocation or 0),
                   committed=mbi.size, resident=resident, private_resident=private_resident,
                   kind=kind, file=name, protection=hex(mbi.protect),
                   resident_runs=[dict(address=hex(r['start']), size=r['end']-r['start'], private=r['private']) for r in runs])
        regions.append(row)
        totals[kind].update({key: row[key] for key in ('committed', 'resident', 'private_resident')})
finally:
    k.CloseHandle(handle)
report = dict(schema_version=1, pid=args.pid, process_creation_time=creation_ticks,
              process_start_utc=process_start_utc, scan_started_utc=started,
              time=datetime.datetime.now(datetime.timezone.utc).isoformat(), duration_seconds=time.perf_counter()-monotonic_start,
              semantics='Non-atomic VirtualQueryEx/QueryWorkingSetEx snapshot; allocation type is not allocating subsystem. No target pages read or forced resident.',
              totals=dict(totals), regions=regions)
args.output.write_text(json.dumps(report, indent=2))
print(json.dumps(dict(pid=args.pid, duration_seconds=report['duration_seconds'], totals=report['totals'], largest_resident_regions=sorted(regions, key=lambda r: r['resident'], reverse=True)[:args.top]), indent=2))
