# Search service setup and restart recovery

> This document describes optional or historical companion tooling. The current native on-device mode does not require this service or a temporary tunnel; see [the main README](../README.md). Its fixed native source profile and image limitations are separate.

The Chromatic fetches results through its own Wi-Fi from the Rust companion. Running the browser preview on a Mac does not configure the handheld. The device needs a reachable HTTPS origin, a valid certificate, and its native synchronized clock.

An experimental device previously kept its origin only in RAM. Power cycling, firmware updates, and opening the development USB diagnostics could reset the MCU and clear that origin. The result was `Search service not configured.` even while the companion and temporary tunnel were working.

## Configure the handheld

Use the authorized development console on the intended Chromatic:

```text
search_endpoint https://your-reachable-host.example --save
search_endpoint status
```

`--save` explicitly persists the origin in the `pocket_search` NVS namespace. Successful configuration requires a commit and matching readback. Boot reads and validates this saved origin. An unchanged origin does not trigger another write. Status reports `configured`, `restore_on_restart`, `saving`, and `storage_error`, without disclosing the hostname, query, or Wi-Fi credentials.

The original `search_endpoint https://hostname` form remains a RAM-only override for temporary tests. It does not overwrite a saved origin; the previously saved origin returns on restart. A rejected input or failed save leaves the active RAM origin unchanged. Storage failures may still require an explicit retry and inspection; this is not a power-loss transactional storage guarantee.

Only bounded ASCII HTTPS hostnames are accepted. Credentials, paths, ports, query strings, fragments, malformed labels, and oversized values are rejected. Missing or corrupt saved data leaves the service unconfigured. Startup does not erase NVS or reset Wi-Fi settings to recover a storage error. NVS I/O occurs outside the network spinlock, and configuration reserves the single request slot while saving.

## Keep the companion reachable

Use the existing owned sample service with the original sample documents and registered images. Its local preview normally listens at `127.0.0.1:8828`. A development tunnel exposes only that read-only fixture service when the operator has authorized it. Private collections and credential files must stay outside it.

Saving a temporary tunnel address does not make it permanent. Both the companion and tunnel must remain running. If the tunnel address changes, configure and save the new origin. Do not disable certificate verification to recover connectivity. A permanent deployment and automatic host service startup remain separate work.

## Verification scope

The host C test exercises the actual origin storage implementation with a controlled NVS fixture: cold-start restoration, missing/corrupt/oversized records, hostname bounds, failed open/set/commit/read, and unchanged-save write suppression. This fixture is not physical flash evidence. The full MCU build and physical request/restart evidence must be recorded separately. USB diagnostic inputs are synthetic; native Wi-Fi requests and MCU render-buffer readbacks do not prove physical button automation, LCD scanout, or release qualification.
