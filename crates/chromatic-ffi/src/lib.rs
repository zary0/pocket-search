// SPDX-License-Identifier: GPL-3.0-only
//! C boundary for the existing allocation-free Rust decoder. Offsets borrow the caller's buffer.
#![cfg_attr(feature = "native", no_std)]
use chromatic_client::{MAX_PACKET_BYTES, Packet, TEXT_ROWS};

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
        ..View::default()
    };
    for (i, line) in packet.lines[..packet.line_count].iter().enumerate() {
        view.lines[i] = text(line);
    }
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
        assert_eq!(core::mem::size_of::<View>(), 44);
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
