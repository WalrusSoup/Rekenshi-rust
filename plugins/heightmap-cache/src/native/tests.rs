use super::*;
use rekenshi_plugin_api::OK;
use std::ffi::CString;
extern "C" fn log(_: u64, message: *const u8, len: usize) {
    // SAFETY: Host logging contract supplies a readable slice for the call.
    let bytes = unsafe { std::slice::from_raw_parts(message, len) };
    println!("{}", String::from_utf8_lossy(bytes));
}
extern "C" fn resolve(_: *const c_char, _: *const c_char) -> *const c_void {
    std::ptr::null()
}
extern "C" fn claim(_: u64, _: *const c_void) -> i32 {
    OK
}
extern "C" fn calls() -> u64 {
    0
}
fn host() -> HostV1 {
    // SAFETY: Test callbacks consume buffers synchronously, return no exports,
    // and treat hook targets as opaque; all callbacks are process-lived.
    unsafe { HostV1::new(41, log, resolve, claim, claim, calls) }
}
struct Reader {
    ptr: *mut c_void,
    destroy: unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void,
}
impl Drop for Reader {
    fn drop(&mut self) {
        // SAFETY: This test owns the reader allocated by native Open. The
        // fingerprinted deleting destructor closes it and frees its native allocation.
        unsafe { (self.destroy)(self.ptr, 1) };
    }
}
#[test]
#[ignore = "requires REKENSHI_TEST_GAME and the installed terrain DLL/fullmap TIFF"]
fn mapped_reads_match_native_pixels_rectangles_edges_and_lifecycle() {
    let game = PathBuf::from(std::env::var_os("REKENSHI_TEST_GAME").expect("REKENSHI_TEST_GAME"));
    rekenshi_native::load_pinned(&game.join("OgreMain_x64.dll")).unwrap();
    start_bindings(host(), &game).unwrap();
    let hooks = HOOKS.get().unwrap();
    let base = hooks.targets[0] - 0x20880;
    let path = CString::new(game.join("data/newland/land/fullmap.tif").to_str().unwrap()).unwrap();
    // SAFETY: The hash-gated open/deleting-destructor/pixel/rectangle ABIs
    // were independently inspected. Inputs and output arrays live through calls.
    let (open, pixel, rectangle, destroy) = unsafe {
        (
            std::mem::transmute::<usize, Open>(hooks.targets[0]),
            std::mem::transmute::<usize, Pixel>(hooks.targets[2]),
            std::mem::transmute::<usize, Rectangle>(hooks.targets[3]),
            std::mem::transmute::<usize, unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void>(
                base + 0x143f0,
            ),
        )
    };
    // SAFETY: Terminated existing TIFF path, mode 1 is the native read-only mode.
    let ptr = unsafe { open(path.as_ptr(), 1) };
    assert!(!ptr.is_null());
    let reader = Reader { ptr, destroy };
    assert!(
        SESSION
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .streams
            .contains_key(&(ptr as usize))
    );
    let (width, height) = {
        let session = SESSION.lock().unwrap();
        let map = &session.as_ref().unwrap().heightmap;
        (map.layout.width as i32, map.layout.height as i32)
    };
    for eager in [false, true] {
        configure(!eager, eager).unwrap();
        let before = reads();
        for i in 0..1024 {
            let x = ((i * 7919) % width).min(width - 1);
            let y = ((i * 3571) % height).min(height - 1);
            let (mut vanilla, mut cached) = (0u16, 0u16);
            // SAFETY: Native live reader, in-bounds coordinates, two writable bytes.
            unsafe {
                let a = hooks
                    .pixel
                    .call(ptr, x, y, (&mut vanilla as *mut u16).cast());
                let b = pixel(ptr, x, y, (&mut cached as *mut u16).cast());
                assert_eq!((b, cached), (a, vanilla), "pixel {x},{y}");
            }
        }
        assert_eq!(reads() - before, 1024);
        for (x, y, w, h) in [
            (0, 0, 4, 3),
            (width - 4, height - 3, 4, 3),
            (-1, -1, 4, 3),
            (width - 2, height - 2, 4, 3),
        ] {
            let (mut vanilla, mut cached) = ([0xa5u16; 12], [0xa5u16; 12]);
            let before = reads();
            // SAFETY: Output arrays have w*h samples; edge rectangles are valid
            // requests of the inspected native clipping implementation.
            unsafe {
                let a = hooks
                    .rectangle
                    .call(ptr, x, y, w, h, vanilla.as_mut_ptr().cast());
                let b = rectangle(ptr, x, y, w, h, cached.as_mut_ptr().cast());
                assert_eq!((b, cached), (a, vanilla), "rectangle {x},{y}");
            }
            assert_eq!(
                reads() - before,
                u64::from(x >= 0 && y >= 0 && x + w <= width && y + h <= height)
            );
        }
    }
    let eager = heightmap(ptr).unwrap();
    assert!(eager.eager_bytes() > 512 * 1024 * 1024);
    let weak = Arc::downgrade(&eager);
    configure(false, true).unwrap();
    assert!(
        Arc::ptr_eq(&eager, &heightmap(ptr).unwrap()),
        "repeated updates must reuse the copy"
    );
    configure(true, false).unwrap();
    assert_eq!(heightmap(ptr).unwrap().eager_bytes(), 0);
    assert_eq!(eager.row(0, 0, 16), heightmap(ptr).unwrap().row(0, 0, 16));
    drop(eager);
    assert!(
        weak.upgrade().is_none(),
        "disabling releases the copy after active reads finish"
    );
    configure(false, false).unwrap();
    let before = reads();
    let mut pixel_out = 0u16;
    // SAFETY: Native reader and two writable output bytes.
    unsafe { pixel(ptr, 0, 0, (&mut pixel_out as *mut u16).cast()) };
    assert_eq!(reads(), before);
    drop(reader);
    assert!(SESSION.lock().unwrap().as_ref().unwrap().streams.is_empty());
    stop().unwrap();
    start_bindings(host(), &game).unwrap();
    configure(false, true).unwrap();
    let eager = Arc::downgrade(
        SESSION
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .gigacache
            .as_ref()
            .unwrap(),
    );
    stop().unwrap();
    assert!(eager.upgrade().is_none(), "shutdown releases Gigacache");
}
