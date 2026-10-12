# Native retained-reader measurements

These are development observations on one Chromatic v4.6, FPGA 18.14, ESP32 at
160 MHz with no PSRAM. The installed MCU application is `0.14.6`, built from
`39607f466dd6f65e14c917c83702251b287b6c00`, 1,993,200 bytes, SHA-256
`89bd58225718a20595ca3776234ad75d2c63782cfc73f0e092fcd3808499848f`.
Application read-back matches. The existing partition table, NVS and retained
corpus were preserved during installation; no bootloader, FPGA or eFuse was
modified. Two fresh macOS output directories produce identical application bytes.
Isolated Linux native CI passes; cross-host byte identity is not established.

The [sanitized numeric record](evidence/native-retained-content/verification.json)
retains revision and workload identity. Controls are queued through the console;
the picture observations are MCU render-buffer readbacks. Neither establishes
independent physical switch/LCD/video behavior. USB supplies power. This record
does not satisfy the ten-case release gate, estimate battery life, or replace the
dedicated lab.

## CPU comparison

Thirty warm comparisons on core 0 encode the **same first retained viewport** and
require byte-identical 192-byte PFW packets. Both paths run inside the same firmware
and frequency: the reference repeats complete layout counting, while the optimized
path uses the immutable document's cached count.

| Work | Total for 30 repetitions | Mean |
| --- | ---: | ---: |
| Reference viewport encoding | 811,601 us | 27,053 us |
| Cached-count viewport encoding | 30,533 us | 1,018 us |
| Local packet decoder | 1,044,947 CPU cycles | 34,832 cycles, about 218 us at 160 MHz |

The encoding comparison is about 26.6 times faster for this window. It does not
claim that networking, arbitrary later windows or the whole application is 26.6
times faster. The decoder/core are allocator-free; zero decoder allocation comes
from that code boundary, not a whole-system allocator trace. Navigation still
scans to the requested window. The used Rust state is 21,936 bytes in a fixed
22,272-byte reservation.

## Actual native GUI windows

Each thirty-second window observes the current reader on core 1 without installing
a synthetic display scene. Wi-Fi is off and no screenshot is requested during the
window. The earlier revision first stops redundant immutable-background assignments;
the latest also skips only the occluded battery widget's paint while the search tab
is visible. Battery sampling and power management remain separate. Returning to the
System tab and back still produces normal content.

| Workload | Frames | DMA bytes | Total GUI handler time | Window minimum free heap |
| --- | ---: | ---: | ---: | ---: |
| `0.14.5` unchanged reader | 504 | 23,224,320 | 15,492,393 us | 96,556 bytes |
| `0.14.6` unchanged reader | 0 | 0 | 549,155 us | 96,784 bytes |
| `0.14.6` four Right, then four Left events | 16 | 737,280 | 969,868 us | 86,196 bytes |

The navigation window's render-start to final-DMA mean is 27,059 us; its maximum
is 40,207 us. Two frames can occur around one queued navigation event as loading
and content change. These are driver timings, not LCD scanout latency. The 15,360-byte
draw band remains allocated. Idle measurement demonstrates stopped redundant frame
transfers; power/current draw and battery savings still need separate measurements.

## Native network and pixels

After native SNTP and verified HTTPS, the latest device fetches a W3C People
baseline JPEG, decodes/resizes it and publishes 10,240 bytes of RGB565 pixels for
the 64 x 80 image viewer. The run transfers 5,746 source bytes including its robots
request, takes 4,155 ms end to end, and spends 18,961 us in decode/resize. Sampled
minimum free heap is 45,316 bytes, largest free block 40,960 bytes, worker stack
headroom 3,104 bytes. Sampling is not an allocation-by-allocation TLS peak trace.
No Mac search service, page proxy or source-image resizing participates. A later
complete thirty-second window covering verified image acquisition, decode/resize,
viewer and B return records an actual local heap minimum of 43,588 bytes, three
frames and 138,240 DMA bytes. Its GUI stack headroom is 4,020 bytes.

With Wi-Fi disabled, image loading shows a failure notice; B restores cached text.
Cancelling a live image request returns error 9 and leaves the reader responsive;
an immediate retry fetches robots and image successfully, publishes 10,240 pixels
and B restores the text. These controls are console-queued, not physical-switch
evidence. The same retained four-document corpus loads after restart with generation 9,
zero storage failures and zero socket requests, and remains searchable/readable
offline. The latest explicit update completes four conditional requests with zero HTML-body
transfer and final HTTP status 304, saving generation 13 with no storage failure.
The earlier `0.14.3` observation retains its own revision. Neither is a deterministic
malformed/timeout/update-200 fixture qualification.

The image format remains bounded baseline JPEG. Progressive JPEG is explicitly
rejected; PNG and other formats are pending. Original public-site photographs and
device/private configuration are not copied into this numeric evidence directory.

## Endurance and remaining acceptance

The latest USB-powered synthetic-control/offline-reader run completes 1,800.06
seconds and 3,760 queued controls. Sampled free heap is at least 95,724 bytes and
ends at 95,748 bytes, with zero socket requests, zero storage failures and retained
generation 9. The exact samples are in the numeric evidence. It traverses all 120 positions of one
retained document repeatedly, checks four loaded documents, zero socket requests,
unchanged storage generation, memory and UI responsiveness. This workload does
not cover thirty minutes of repeated TLS/image decoding, physical battery behavior
or flash interruption. Those cases remain mandatory in the dedicated physical lab.

## Native image formats on 0.14.9

The latest image candidate, source `91022671bb21e60cf627466c1d367e14aae9f2e0`,
completes a USB-powered 1,810.676-second original-source image run with 160
JPEG/PNG acquisitions and decodes, cancellation/retry/back, verified native TLS,
zero unexpected errors/resets and zero persistent-store failures. This supersedes
the older baseline-only image-format boundary, without changing those older
measurements. See [format-specific memory/time and qualification](native-image-formats.md)
and [candidate-bound numeric evidence](evidence/native-images/verification.json).
Independent battery, physical switches/video and isolated CI E2E remain pending.
