// SPDX-License-Identifier: GPL-3.0-only
//! C boundary for the existing allocation-free Rust decoder. Offsets borrow the caller's buffer.
#![cfg_attr(feature = "native", no_std)]
use chromatic_client::local::LocalSearch;
use chromatic_client::{MAX_PACKET_BYTES, Packet, TEXT_ROWS};

#[unsafe(no_mangle)]
pub extern "C" fn ps_local_state_bytes() -> usize {
    core::mem::size_of::<LocalSearch>()
}

fn state_valid(state: *mut u8, size: usize) -> bool {
    !state.is_null()
        && size >= core::mem::size_of::<LocalSearch>()
        && (state as usize) % core::mem::align_of::<LocalSearch>() == 0
}
/// Initialize one aligned opaque on-device state. No allocation is performed.
/// # Safety
/// State owns `size` writable bytes with LocalSearch alignment. Every later call
/// uses the same initialized, exclusively owned region. No other call, ISR or
/// thread may access it concurrently. Input/output buffers must not overlap it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_init(state: *mut u8, size: usize) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    // LocalSearch contains only integers and integer arrays: zero is valid.
    unsafe {
        core::ptr::write_bytes(state, 0, core::mem::size_of::<LocalSearch>());
    }
    unsafe { &mut *state.cast::<LocalSearch>() }.seed();
    0
}
/// # Safety
/// Initialized exclusive state as for ps_local_init; out owns length writable,
/// non-overlapping bytes. A successful call begins one unpublished document.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_next(
    state: *mut u8,
    size: usize,
    out: *mut u8,
    length: usize,
) -> i32 {
    if !state_valid(state, size) || out.is_null() || length != 128 {
        return -1;
    }
    unsafe { &mut *state.cast::<LocalSearch>() }
        .next_url(unsafe { core::slice::from_raw_parts_mut(out, length) }) as i32
}
/// # Safety
/// Initialized exclusive state; bytes owns length readable bytes, disjoint from
/// state. Feed only body data of the current allowed, verified HTTPS document.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_feed(
    state: *mut u8,
    size: usize,
    bytes: *const u8,
    length: usize,
) -> i32 {
    if !state_valid(state, size) || bytes.is_null() || length > 512 {
        return -1;
    }
    unsafe { &mut *state.cast::<LocalSearch>() }
        .feed(unsafe { core::slice::from_raw_parts(bytes, length) });
    0
}
/// # Safety
/// Initialized exclusive state. Complete must be 1 only after transport, status,
/// content type, body limit and robots checks all pass and HTTP EOF is complete.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_finish(state: *mut u8, size: usize, complete: i32) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    i32::from(unsafe { &mut *state.cast::<LocalSearch>() }.finish(complete == 1))
}
/// # Safety
/// Initialized exclusive state; out owns 512 writable bytes, query owns length
/// readable UTF-8 bytes. All regions are disjoint. Kind 1 searches; kind 2 reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_packet(
    state: *mut u8,
    size: usize,
    kind: u32,
    document: u32,
    page: u32,
    query: *const u8,
    length: usize,
    out: *mut u8,
) -> i32 {
    if !state_valid(state, size) || query.is_null() || out.is_null() || length > 96 {
        return -1;
    }
    let Ok(query) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(query, length) })
    else {
        return -1;
    };
    let index = unsafe { &*state.cast::<LocalSearch>() };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 512) };
    (match kind {
        1 => index.search_packet(query, page as usize, out),
        2 => index.reader_packet(document as usize, page as usize, out),
        _ => 0,
    }) as i32
}
/// # Safety
/// Initialized exclusive state; no concurrent access, including readers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_count(state: *mut u8, size: usize) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    unsafe { &*state.cast::<LocalSearch>() }.count() as i32
}
/// # Safety
/// Bytes owns length readable bytes. Path owns path_length readable UTF-8 bytes;
/// neither buffer is mutated. Conservative unsupported robots files deny access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_robots(
    bytes: *const u8,
    length: usize,
    path: *const u8,
    path_length: usize,
) -> i32 {
    if bytes.is_null() || path.is_null() || length > 2048 || path_length > 128 {
        return 0;
    }
    let Ok(path) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(path, path_length) })
    else {
        return 0;
    };
    i32::from(chromatic_client::local::robots_allowed(
        unsafe { core::slice::from_raw_parts(bytes, length) },
        path,
    ))
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Text {
    pub offset: u16,
    pub length: u16,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct View {
    pub document_id: u32,
    pub page_index: u16,
    pub page_count: u16,
    pub kind: u8,
    pub line_count: u8,
    pub reserved: u16,
    pub title: Text,
    pub lines: [Text; TEXT_ROWS],
    pub link_target: u32,
}

/// Validate a PCS1 packet and return text offsets, leaving output unchanged on failure.
///
/// # Safety
/// `bytes` must point to `length` readable bytes, and `out` to one aligned,
/// writable View. The regions must not overlap. The caller retains the packet
/// buffer unchanged for every later use of the returned offsets.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_packet_decode(bytes: *const u8, length: usize, out: *mut View) -> i32 {
    if bytes.is_null()
        || out.is_null()
        || !(20..=MAX_PACKET_BYTES).contains(&length)
        || (out as usize) % core::mem::align_of::<View>() != 0
    {
        return -1;
    }
    // The only foreign-memory reads are under the documented C caller contract.
    let data = unsafe { core::slice::from_raw_parts(bytes, length) };
    let Ok(packet) = Packet::decode(data) else {
        return -2;
    };
    let text = |s: &str| Text {
        offset: (s.as_ptr() as usize - data.as_ptr() as usize) as u16,
        length: s.len() as u16,
    };
    let mut view = View {
        document_id: packet.document_id,
        page_index: packet.page_index,
        page_count: packet.page_count,
        kind: packet.kind as u8,
        line_count: packet.line_count as u8,
        title: text(packet.title),
        link_target: packet.link_target,
        ..View::default()
    };
    for (i, line) in packet.lines[..packet.line_count].iter().enumerate() {
        view.lines[i] = text(line);
    }
    unsafe { out.write(view) };
    0
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ImageView {
    pub document_id: u32,
    pub page_index: u16,
    pub width: u16,
    pub height: u16,
    pub y: u16,
    pub rows: u16,
    pub pixels_offset: u16,
    pub pixels_length: u16,
    pub reserved: u16,
}
/// Validate one PCI1 stripe without changing output on failure.
/// # Safety
/// Input is readable for length bytes; output is aligned, writable, non-overlapping.
/// The caller retains the input unchanged while using the returned pixel offset.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_image_decode(
    bytes: *const u8,
    length: usize,
    out: *mut ImageView,
) -> i32 {
    if bytes.is_null()
        || out.is_null()
        || !(24..=chromatic_client::MAX_IMAGE_PACKET_BYTES).contains(&length)
        || (out as usize) % core::mem::align_of::<ImageView>() != 0
    {
        return -1;
    }
    let input = unsafe { core::slice::from_raw_parts(bytes, length) };
    let Ok(stripe) = chromatic_client::ImageStripe::decode(input) else {
        return -2;
    };
    let view = ImageView {
        document_id: stripe.document_id,
        page_index: stripe.page_index,
        width: stripe.width,
        height: stripe.height,
        y: stripe.y,
        rows: stripe.rows,
        pixels_offset: 24,
        pixels_length: stripe.pixels.len() as u16,
        reserved: 0,
    };
    unsafe { out.write(view) };
    0
}

#[cfg(feature = "native")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe extern "C" {
        fn abort() -> !;
    }
    unsafe { abort() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_opaque_state_ffi_streaming_and_refusals() {
        #[repr(align(8))]
        struct State([u8; chromatic_client::local::STATE_BUDGET]);
        let mut state = State([255; chromatic_client::local::STATE_BUDGET]);
        let ptr = state.0.as_mut_ptr();
        let size = state.0.len();
        let mut url = [0; 128];
        let mut packet = [0; 512];
        assert!(ps_local_state_bytes() <= size);
        unsafe {
            assert_eq!(ps_local_init(ptr.add(1), size - 1), -1);
            assert_eq!(ps_local_init(ptr, 1), -1);
            assert_eq!(ps_local_init(ptr, size), 0);
            assert_eq!(ps_local_count(ptr, size), 0);
            let len = ps_local_next(ptr, size, url.as_mut_ptr(), url.len());
            assert_eq!(
                &url[..len as usize],
                chromatic_client::local::SEED.as_bytes()
            );
            for b in b"<title>Local AI</title><p>Real AI page text.</p>" {
                assert_eq!(ps_local_feed(ptr, size, b, 1), 0);
            }
            assert_eq!(ps_local_finish(ptr, size, 1), 1);
            assert_eq!(ps_local_count(ptr, size), 1);
            let len = ps_local_packet(ptr, size, 1, 0, 0, b"AI".as_ptr(), 2, packet.as_mut_ptr());
            let view = Packet::decode(&packet[..len as usize]).unwrap();
            assert_eq!(view.document_id, 1);
            assert_eq!(view.title, "Local AI");
            assert_eq!(
                ps_local_packet(ptr, size, 2, 1, 99, b"".as_ptr(), 0, packet.as_mut_ptr()),
                0
            );
            assert_eq!(ps_local_feed(ptr, size, core::ptr::null(), 1), -1);
            assert_eq!(
                ps_local_packet(ptr, size, 1, 0, 0, [255].as_ptr(), 1, packet.as_mut_ptr()),
                -1
            );
            assert_eq!(ps_local_init(ptr, size), 0);
            assert_eq!(ps_local_count(ptr, size), 0);
        }
    }
    #[test]
    fn ffi_rejects_invalid_input_without_changing_output() {
        let bytes = [0u8; 20];
        let mut view = View {
            document_id: 42,
            ..View::default()
        };
        let before = view;
        assert_eq!(
            unsafe { ps_packet_decode(bytes.as_ptr(), bytes.len(), &mut view) },
            -2
        );
        assert_eq!(view, before);
        assert_eq!(
            unsafe { ps_packet_decode(core::ptr::null(), 20, &mut view) },
            -1
        );
    }
    #[test]
    fn ffi_layout_and_borrowed_text_match_c() {
        assert_eq!(core::mem::size_of::<View>(), 48);
        assert_eq!(core::mem::size_of::<ImageView>(), 20);
        let mut bytes =
            b"PCS1\x01\x00\x01\x01\x01\x00\x00\x00\x00\x00\x01\x00\x03\x00\x00\x00X\x01Y".to_vec();
        let mut view = View::default();
        assert_eq!(
            unsafe { ps_packet_decode(bytes.as_ptr(), bytes.len(), &mut view) },
            0
        );
        assert_eq!(
            view.title,
            Text {
                offset: 20,
                length: 1
            }
        );
        assert_eq!(
            view.lines[0],
            Text {
                offset: 22,
                length: 1
            }
        );
        bytes[22] = 255;
        assert_ne!(
            unsafe { ps_packet_decode(bytes.as_ptr(), bytes.len(), &mut view) },
            0
        );
    }
}
