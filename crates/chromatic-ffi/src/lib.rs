// SPDX-License-Identifier: GPL-3.0-only
//! C boundary for the existing allocation-free Rust decoder. Offsets borrow the caller's buffer.
#![cfg_attr(feature = "native", no_std)]
use chromatic_client::local::LocalSearch;
use chromatic_client::{MAX_PACKET_BYTES, Packet, TEXT_ROWS};

#[unsafe(no_mangle)]
pub extern "C" fn ps_local_state_bytes() -> usize {
    core::mem::size_of::<LocalSearch>()
}

#[unsafe(no_mangle)]
pub extern "C" fn ps_local_backup_bytes() -> usize {
    core::mem::size_of::<chromatic_client::local::SavedDocument>()
}
/// # Safety
/// Exclusive initialized state; saved is a zero-initialized or previously written,
/// aligned region of ps_local_backup_bytes writable bytes, disjoint from state.
/// Restore may be 1 only after this function has initialized that same snapshot;
/// the adapter must not modify its contents. The current document stays active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_backup(
    state: *mut u8,
    size: usize,
    saved: *mut u8,
    length: usize,
    restore: i32,
) -> i32 {
    use chromatic_client::local::SavedDocument;
    if !state_valid(state, size)
        || saved.is_null()
        || length != core::mem::size_of::<SavedDocument>()
        || (saved as usize) % core::mem::align_of::<SavedDocument>() != 0
    {
        return -1;
    }
    let state = unsafe { &mut *state.cast::<LocalSearch>() };
    if restore == 1 {
        state.restore_active(unsafe { &*saved.cast::<SavedDocument>() });
    } else {
        if !state.save_active(unsafe { &mut *saved.cast::<SavedDocument>() }) {
            return -2;
        }
    }
    0
}

/// # Safety
/// Exclusive initialized state; url owns length readable bytes, disjoint from state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_css_cached(
    state: *mut u8,
    size: usize,
    url: *const u8,
    length: usize,
) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    if url.is_null() || length > 255 {
        return -1;
    }
    let index = unsafe { &*state.cast::<LocalSearch>() };
    i32::from(
        index.css_cached()
            && index.resource(false).as_bytes()
                == unsafe { core::slice::from_raw_parts(url, length) },
    )
}
/// # Safety
/// Exclusive initialized state, initialized as for ps_local_init.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_css_count(state: *mut u8, size: usize) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    unsafe { &*state.cast::<LocalSearch>() }.css_count() as i32
}
/// # Safety
/// Exclusive initialized state; out owns 256 writable bytes, disjoint from state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_resource(
    state: *mut u8,
    size: usize,
    font: i32,
    out: *mut u8,
) -> i32 {
    if !state_valid(state, size) || out.is_null() {
        return -1;
    }
    let s = unsafe { &*state.cast::<LocalSearch>() }
        .resource(font != 0)
        .as_bytes();
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, s.len());
        *out.add(s.len()) = 0;
    }
    s.len() as i32
}
/// # Safety
/// Exclusive initialized state; bytes owns length readable bytes (<=3072), base
/// owns base_len readable UTF-8 bytes (<=255); all regions must be disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_css(
    state: *mut u8,
    size: usize,
    bytes: *const u8,
    length: usize,
    base: *const u8,
    base_len: usize,
) -> i32 {
    if !state_valid(state, size)
        || bytes.is_null()
        || base.is_null()
        || length > chromatic_client::css::BYTES
        || base_len > 255
    {
        return -1;
    }
    let Ok(base) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(base, base_len) })
    else {
        return -1;
    };
    let state = unsafe { &mut *state.cast::<LocalSearch>() };
    state.stylesheet(unsafe { core::slice::from_raw_parts(bytes, length) }, base);
    state.css_count() as i32
}
/// # Safety
/// Exclusive initialized state. Reparse only the active unpublished document.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_reparse(state: *mut u8, size: usize, ready: i32) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    let state = unsafe { &mut *state.cast::<LocalSearch>() };
    if ready == 1 {
        state.font_ready();
    }
    state.reparse();
    0
}
/// # Safety
/// bytes owns length readable bytes (<=24576); atlas owns 867 writable bytes;
/// buffers are disjoint and exclusively owned until return. Publish only on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_webfont(bytes: *const u8, length: usize, atlas: *mut u8) -> i32 {
    if bytes.is_null() || atlas.is_null() || length > chromatic_client::webfont::LIMIT {
        return -1;
    }
    chromatic_client::webfont::rasterize(
        unsafe { core::slice::from_raw_parts(bytes, length) },
        unsafe { &mut *atlas.cast::<[u8; chromatic_client::webfont::ATLAS_BYTES]>() },
    ) as i32
}
/// # Safety
/// out owns 512 writable bytes, exclusively.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_webfont_probe(out: *mut u8) -> i32 {
    if out.is_null() {
        return -1;
    }
    chromatic_client::rich::webfont_probe(unsafe { core::slice::from_raw_parts_mut(out, 512) })
        as i32
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
/// Exclusively owned initialized state as for ps_local_init.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_cancel_transfer(state: *mut u8, size: usize) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    unsafe { &mut *state.cast::<LocalSearch>() }.cancel_transfer();
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn ps_local_profiles() -> u32 {
    chromatic_client::local::PROFILES.len() as u32
}
fn saved_valid(saved: *mut u8, length: usize) -> bool {
    !saved.is_null()
        && length == ps_local_backup_bytes()
        && (saved as usize) % core::mem::align_of::<chromatic_client::local::SavedDocument>() == 0
}
/// # Safety
/// State follows ps_local_init. Saved is a zeroed aligned region of the exact
/// backup size; URL exclusively owns 128 bytes. All regions are disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_update_begin(
    state: *mut u8,
    size: usize,
    id: u32,
    saved: *mut u8,
    length: usize,
    url: *mut u8,
) -> i32 {
    if !state_valid(state, size) || !saved_valid(saved, length) || url.is_null() {
        return -1;
    }
    unsafe { &mut *state.cast::<LocalSearch>() }.begin_update(
        id as usize,
        unsafe { &mut *saved.cast() },
        unsafe { core::slice::from_raw_parts_mut(url, 128) },
    ) as i32
}
/// # Safety
/// Saved was initialized by ps_local_update_begin and remains unchanged; output
/// owns 96 writable bytes disjoint from it. Regions are exclusively owned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_update_header(
    saved: *mut u8,
    length: usize,
    modified: i32,
    out: *mut u8,
) -> i32 {
    if !saved_valid(saved, length) || out.is_null() {
        return -1;
    }
    LocalSearch::update_header(unsafe { &*saved.cast() }, modified != 0, unsafe {
        core::slice::from_raw_parts_mut(out, 96)
    }) as i32
}
/// # Safety
/// Exclusive initialized state follows ps_local_init. Saved was initialized by
/// ps_local_update_begin for this active update, is unchanged and disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_update_restore(
    state: *mut u8,
    size: usize,
    saved: *mut u8,
    length: usize,
    unchanged: i32,
    checked: u32,
) -> i32 {
    if !state_valid(state, size) || !saved_valid(saved, length) {
        return -1;
    }
    if unsafe { &mut *state.cast::<LocalSearch>() }.restore_update(
        unsafe { &*saved.cast() },
        unchanged != 0,
        checked,
    ) {
        0
    } else {
        -1
    }
}
/// # Safety
/// State follows ps_local_init. ETag and Modified own their readable lengths,
/// are UTF-8 and exclusively owned, with no overlap with the initialized state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_validators(
    state: *mut u8,
    size: usize,
    etag: *const u8,
    etag_len: usize,
    modified: *const u8,
    modified_len: usize,
) -> i32 {
    if !state_valid(state, size)
        || etag.is_null()
        || modified.is_null()
        || etag_len > 95
        || modified_len > 63
    {
        return -1;
    }
    let Ok(etag) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(etag, etag_len) })
    else {
        return -1;
    };
    let Ok(modified) =
        core::str::from_utf8(unsafe { core::slice::from_raw_parts(modified, modified_len) })
    else {
        return -1;
    };
    if unsafe { &mut *state.cast::<LocalSearch>() }.validators(etag, modified) {
        0
    } else {
        -1
    }
}
/// Maximum serialized retained-content bytes; excludes queries and runtime memory.
#[unsafe(no_mangle)]
pub extern "C" fn ps_local_snapshot_bytes() -> usize {
    chromatic_client::local::SNAPSHOT_BYTES
}
/// # Safety
/// State follows ps_local_init. Output exclusively owns capacity bytes and does
/// not overlap state. No concurrent access to either region.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_snapshot(
    state: *mut u8,
    size: usize,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    if !state_valid(state, size)
        || out.is_null()
        || capacity > chromatic_client::local::SNAPSHOT_BYTES
    {
        return -1;
    }
    unsafe { &*state.cast::<LocalSearch>() }
        .snapshot(unsafe { core::slice::from_raw_parts_mut(out, capacity) })
        .map_or(-1, |n| n as i32)
}
/// # Safety
/// Exclusive initialized empty state follows ps_local_init. Input owns length
/// readable bytes, does not overlap state, and cannot be changed concurrently.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_restore_snapshot(
    state: *mut u8,
    size: usize,
    bytes: *const u8,
    length: usize,
) -> i32 {
    if !state_valid(state, size)
        || bytes.is_null()
        || length > chromatic_client::local::SNAPSHOT_BYTES
    {
        return -1;
    }
    if unsafe { &mut *state.cast::<LocalSearch>() }
        .restore_snapshot(unsafe { core::slice::from_raw_parts(bytes, length) })
    {
        0
    } else {
        -1
    }
}
/// # Safety
/// Exclusively owned initialized state as for ps_local_init.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_pending(state: *mut u8, size: usize) -> i32 {
    if !state_valid(state, size) {
        return -1;
    }
    i32::from(unsafe { &*state.cast::<LocalSearch>() }.pending())
}
/// # Safety
/// Exclusive initialized state as for ps_local_init. Resets collection only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_set_profile(state: *mut u8, size: usize, profile: u32) -> i32 {
    if !state_valid(state, size) || profile > u8::MAX as u32 {
        return -1;
    }
    if unsafe { &mut *state.cast::<LocalSearch>() }.set_profile(profile as u8) {
        0
    } else {
        -1
    }
}
/// # Safety
/// The URL owns length readable bytes. No input is dereferenced when invalid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_allowed(profile: u32, url: *const u8, length: usize) -> i32 {
    if profile > u8::MAX as u32 || url.is_null() || length > 255 {
        return -1;
    }
    let Ok(url) = core::str::from_utf8(unsafe { core::slice::from_raw_parts(url, length) }) else {
        return -1;
    };
    i32::from(chromatic_client::local::allowed_profile_url(
        profile as u8,
        url,
    ))
}
/// # Safety
/// Out owns capacity writable bytes. No other state or input may overlap it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_profile_text(
    profile: u32,
    part: u32,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    let Some(&(name, base, seed, robots)) = chromatic_client::local::PROFILES.get(profile as usize)
    else {
        return -1;
    };
    let text = match part {
        0 => name,
        1 => base,
        2 => seed,
        3 => robots,
        _ => return -1,
    };
    if out.is_null() || capacity <= text.len() {
        return -1;
    }
    let output = unsafe { core::slice::from_raw_parts_mut(out, capacity) };
    output[..text.len()].copy_from_slice(text.as_bytes());
    output[text.len()] = 0;
    text.len() as i32
}
/// # Safety
/// Exclusive initialized state; out owns capacity writable non-overlapping bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_image_url(
    state: *mut u8,
    size: usize,
    id: u32,
    position: u32,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    if !state_valid(state, size) || out.is_null() || capacity < chromatic_client::css::URL {
        return -1;
    }
    let output = unsafe { core::slice::from_raw_parts_mut(out, capacity) };
    let n =
        unsafe { &*state.cast::<LocalSearch>() }.image_url(id as usize, position as usize, output);
    if n > 0 && n < capacity {
        output[n] = 0;
        n as i32
    } else {
        0
    }
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
/// Exclusive initialized state. Record only a fully fetched verified HTML response.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_local_metadata(
    state: *mut u8,
    size: usize,
    bytes: u32,
    fetched_at: u32,
    status: u32,
) -> i32 {
    if !state_valid(state, size) || status > u16::MAX as u32 {
        return -1;
    }
    if unsafe { &mut *state.cast::<LocalSearch>() }.metadata(bytes, fetched_at, status as u16) {
        0
    } else {
        -1
    }
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
        2 => index.flow_packet(document as usize, page as usize, out),
        4 => index.info_packet(document as usize, page as usize, out),
        6 => index.flow_packet_reference(document as usize, page as usize, out),
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
    if bytes.is_null() || path.is_null() || length > 4096 || path_length > 256 {
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

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct RichRow {
    pub text: Text,
    pub styles: Text,
    pub y: u8,
    pub height: u8,
    pub glyphs: u8,
    pub reserved: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct RichView {
    pub document_id: u32,
    pub position: u16,
    pub positions: u16,
    pub title: Text,
    pub row_count: u8,
    pub reserved: [u8; 3],
    pub rows: [RichRow; chromatic_client::rich::ROWS],
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct FlowRow {
    pub text: Text,
    pub codes: Text,
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub text_y: u8,
    pub stroke: u8,
    pub edges: u8,
    pub flags: u8,
    pub background: u16,
    pub border: u16,
    pub box_x: u8,
    pub box_width: u8,
    pub reserved: [u8; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct FlowView {
    pub document_id: u32,
    pub position: u16,
    pub positions: u16,
    pub link_target: u32,
    pub title: Text,
    pub palette: Text,
    pub row_count: u8,
    pub limited: u8,
    pub reserved: [u8; 2],
    pub rows: [FlowRow; chromatic_client::flow::ROWS],
}
/// Original diagnostic viewport; does not touch the retained search collection.
/// # Safety
/// Out owns 512 exclusively writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_flow_probe(out: *mut u8, position: u32) -> i32 {
    if out.is_null() {
        return -1;
    }
    chromatic_client::flow::probe(position as usize, unsafe {
        core::slice::from_raw_parts_mut(out, 512)
    }) as i32
}
/// Validate a bounded PFW1 viewport. Leave output unchanged on failure.
/// # Safety
/// Input owns length readable bytes; output is aligned, writable and disjoint.
/// Keep input unchanged while any returned offsets are in use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_flow_decode(
    bytes: *const u8,
    length: usize,
    out: *mut FlowView,
) -> i32 {
    if bytes.is_null()
        || out.is_null()
        || !(24..=MAX_PACKET_BYTES).contains(&length)
        || (out as usize) % core::mem::align_of::<FlowView>() != 0
    {
        return -1;
    }
    let data = unsafe { core::slice::from_raw_parts(bytes, length) };
    let Ok(packet) = chromatic_client::flow::Packet::decode(data) else {
        return -2;
    };
    let range = |b: &[u8]| Text {
        offset: (b.as_ptr() as usize - data.as_ptr() as usize) as u16,
        length: b.len() as u16,
    };
    let mut view = FlowView {
        document_id: packet.document_id,
        position: packet.position,
        positions: packet.positions,
        link_target: packet.link_target,
        title: range(packet.title.as_bytes()),
        palette: range(packet.palette),
        row_count: packet.row_count as u8,
        limited: data[19],
        ..FlowView::default()
    };
    for (i, row) in packet.rows[..packet.row_count].iter().enumerate() {
        view.rows[i] = FlowRow {
            text: range(row.text.as_bytes()),
            codes: range(row.codes),
            x: row.x,
            y: row.y,
            width: row.width,
            height: row.height,
            text_y: row.text_y,
            stroke: row.stroke,
            edges: row.edges,
            flags: row.flags,
            background: row.background,
            border: row.border,
            box_x: row.box_x,
            box_width: row.box_width,
            reserved: [0; 2],
        };
    }
    unsafe { out.write(view) };
    0
}
/// Validate a PSR1 viewport and borrow its text/style offsets.
/// # Safety
/// Input owns length readable bytes; output is aligned, writable and disjoint.
/// Keep input unchanged while any returned offsets are in use.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_rich_decode(
    bytes: *const u8,
    length: usize,
    out: *mut RichView,
) -> i32 {
    if bytes.is_null()
        || out.is_null()
        || !(20..=MAX_PACKET_BYTES).contains(&length)
        || (out as usize) % core::mem::align_of::<RichView>() != 0
    {
        return -1;
    }
    let data = unsafe { core::slice::from_raw_parts(bytes, length) };
    let Ok(packet) = chromatic_client::rich::Packet::decode(data) else {
        return -2;
    };
    let range = |b: &[u8]| Text {
        offset: (b.as_ptr() as usize - data.as_ptr() as usize) as u16,
        length: b.len() as u16,
    };
    let mut view = RichView {
        document_id: packet.document_id,
        position: packet.position,
        positions: packet.positions,
        title: range(packet.title.as_bytes()),
        row_count: packet.row_count as u8,
        ..RichView::default()
    };
    for (i, row) in packet.rows[..packet.row_count].iter().enumerate() {
        view.rows[i] = RichRow {
            text: range(row.text.as_bytes()),
            styles: range(row.styles),
            y: row.y,
            height: row.height,
            glyphs: row.glyphs,
            reserved: 0,
        };
    }
    unsafe { out.write(view) };
    0
}
/// Original calibration viewport; does not touch the search collection.
/// # Safety
/// Out owns 512 writable bytes and is not concurrently accessed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ps_rich_probe(out: *mut u8) -> i32 {
    if out.is_null() {
        return -1;
    }
    chromatic_client::rich::probe(unsafe { core::slice::from_raw_parts_mut(out, 512) }) as i32
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
    fn rich_ffi_borrows_styles_and_rejects_bad_input_without_changing_output() {
        assert_eq!(core::mem::size_of::<RichView>(), 112);
        let mut bytes = [0; 512];
        let mut view = RichView::default();
        unsafe {
            let n = ps_rich_probe(bytes.as_mut_ptr());
            assert!(n > 0);
            assert_eq!(ps_rich_decode(bytes.as_ptr(), n as usize, &mut view), 0);
            assert_eq!(view.rows[0].height, 18);
            assert_eq!(view.rows[1].height, 14);
            let caption = view.rows.iter().find(|r| r.height == 12).unwrap();
            assert_eq!(
                bytes[caption.styles.offset as usize] & 15,
                chromatic_client::rich::CAPTION
            );
            let prior = view;
            bytes[4] = 3;
            assert_eq!(ps_rich_decode(bytes.as_ptr(), n as usize, &mut view), -2);
            assert_eq!(view, prior);
            assert_eq!(ps_rich_decode(core::ptr::null(), n as usize, &mut view), -1);
        }
    }
    #[test]
    fn native_robots_ffi_accepts_large_policy_and_rejects_oversize() {
        let mut rules = [b' '; 4096];
        let prefix = b"User-agent: *\nDisallow: /private\n#";
        rules[..prefix.len()].copy_from_slice(prefix);
        unsafe {
            assert_eq!(
                ps_local_robots(rules.as_ptr(), 3844, b"/People/Berners-Lee/".as_ptr(), 19),
                1
            );
            assert_eq!(
                ps_local_robots(rules.as_ptr(), 4096, b"/private".as_ptr(), 8),
                0
            );
            assert_eq!(ps_local_robots(rules.as_ptr(), 4097, b"/".as_ptr(), 1), 0);
        }
    }
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
    #[test]
    fn flow_abi_borrowing_scrolling_and_invalid_output_contract() {
        assert_eq!(core::mem::size_of::<FlowRow>(), 24);
        assert_eq!(core::mem::size_of::<FlowView>(), 216);
        let mut bytes = [0; 512];
        let mut view = FlowView::default();
        unsafe {
            let n = ps_flow_probe(bytes.as_mut_ptr(), 0);
            assert!(n > 0);
            assert_eq!(ps_flow_decode(bytes.as_ptr(), n as usize, &mut view), 0);
            assert!(view.positions > 1);
            let positions = view.positions;
            for p in 0..positions {
                let n = ps_flow_probe(bytes.as_mut_ptr(), u32::from(p));
                assert!(n > 0);
                assert_eq!(ps_flow_decode(bytes.as_ptr(), n as usize, &mut view), 0);
                assert_eq!(view.position, p);
                for row in &view.rows[..view.row_count as usize] {
                    assert!(row.text.offset as usize + row.text.length as usize <= n as usize);
                    assert!(row.x as usize + row.width as usize <= 160);
                }
            }
            let previous = view;
            bytes[4] = 99;
            assert_eq!(ps_flow_decode(bytes.as_ptr(), 512, &mut view), -2);
            assert_eq!(view, previous);
            assert_eq!(ps_flow_decode(core::ptr::null(), 512, &mut view), -1);
            assert_eq!(ps_flow_probe(bytes.as_mut_ptr(), u32::from(positions)), 0);
        }
    }
}
