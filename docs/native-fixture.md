# Deterministic native source fixture

`scripts/native_fixture.py` serves four original bounded HTML pages and an original
192 x 128 baseline JPEG. The device must collect, extract, index, lay out, decode and
resize them itself. The origin has no search endpoint, page proxy, browser engine,
image conversion, outbound requests or filesystem URL mapping. It forces native resize to 120 x 80 rather than shipping
display-ready pixels. The fixture and its image are GPL-3.0-only; image provenance is in `ci/native-fixture/asset.json`.

The pages contain `PocketLab`, mixed English/Japanese text, headings, paragraph
sizes, strong text, lists, links, an image slot and a definite end marker. They fit
the current retained-content limits. Search for `PocketLab` after collecting all
four pages; use `quartz` for an absent term. Native E2E must check text, ordering,
last-position bounds and actual glyphs rather than assuming a successful request
means a correct display.

`ci/native-fixture/generate.py` is the preferred source for the original bars. It
requires Pillow only to regenerate a new JPEG; serving, CI content checks and the
native device have no Pillow dependency. Encoder versions may change bytes, so
the reviewed committed asset and its checksum remain the fixture's identity.

## Operator provisioning

Use a dedicated fixture origin with a valid public certificate chain accepted by
the device, normal DNS and HTTPS on port 443. The development listener defaults
to loopback port 8443 and always requires a certificate and key. A dedicated lab
ingress can forward port 443 to it. Private keys remain outside the checkout;
restrict access and retain request traces only for the controlled lab. The normal
certificate/hostname checks and native SNTP are mandatory. Do not turn off TLS
validation, add an IP/port exception or pretend that localhost is a device-native
verified connection.

```sh
python3 scripts/native_fixture.py \
  --origin https://fixture.example.org \
  --profile-output /tmp/native-lab-sources.json
```

This creates a **new** profile file and performs the production source-admission
checks. It does not edit project sources, configure a device, grant a runner access
or launch a service. Replace the example origin with the operator-controlled
public origin. Review and commit the corresponding source profile and regenerate
the registry before building the exact qualification candidate. A candidate that
does not admit the fixture's authority cannot use it. Never patch the profile or
installed firmware after establishing candidate identity, impersonate a W3C or
Stanford hostname, or silently substitute a different test image.

```sh
python3 scripts/native_fixture.py \
  --origin https://fixture.example.org \
  --cert /operator-private/fullchain.pem --key /operator-private/key.pem \
  --mode normal --trace /operator-private/new-request-trace.jsonl
```

The trace records only known resource names, status, byte counts and conditional
request presence. It excludes client IPs, arbitrary URLs, query strings, cookies,
authorization values and other headers. A trace and a User-Agent string do not
authenticate a physical device: the trusted lab must correlate the actual native
UI actions and network requests. Keep lab audit records separate from release
artifacts and private device recovery data.

## Repeatable cases

| Mode | Behavior |
| --- | --- |
| `normal` | Four documents, baseline source image, robots policy, ETag/Last-Modified and 304 |
| `changed` | Document 2 changes text and ETag; explicit update returns a complete 200 |
| `timeout` | Document 2 waits 20 seconds, beyond the native eight-second socket deadline |
| `unavailable` | Document 2 returns 503 |
| `oversized` | Document 2 declares 65,537 bytes, beyond the native HTML admission limit |
| `wrong-type` | Document 2 returns the wrong MIME type |
| `truncated` | Document 2 closes before its declared Content-Length |
| `malformed-image` | The source image has only SOI/EOI markers and must be rejected |

Other pages remain available during document-2 fault cases. Restart the lab-owned
listener with the chosen mode, or let the trusted driver control its in-memory
mode between completed requests. There are no remote mode-changing endpoints.
For cancellation, observe B during collection/loading, confirm return to readable
content and retry under `normal`; cancellation must not turn robots admission
into a cached denial. The listener limits concurrent requests to eight and socket
operations to ten seconds, with the deliberate twenty-second fixture delay.

The fixture's Python and loopback HTTP tests verify content, conditional headers,
admission boundaries, fault sizes and trace redaction. They do not qualify native
TLS, power interruption, battery operation, physical switches/video or JPEG entropy
decoding. The origin is implemented but not deployed here; the dedicated runner,
trusted physical driver, approved candidate profile and public HTTPS ingress are
still required by the [physical lab contract](hardware-e2e.md).

## Extended image sources

The `png` and `progressive` modes point the same original HTML at `bars.png` or
`bars-progressive.jpg`. `palette.png` exercises packed indices and transparency.
`extended-assets.json` records original GPL-3.0-only provenance and source hashes.
The PNG contains all five row filters and short IDAT chunks; the progressive
JPEG is 64 x 128. These are encoded source fixtures, never display-ready pixels.
A host HTTP unit test or decoder pass is not native acquisition evidence.
