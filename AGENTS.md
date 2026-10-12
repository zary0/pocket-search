# Repository workflow

- Prepare changes on feature/fix branches and open or update PRs targeting `develop`.
- The maintainer merges PRs manually. Do not merge a PR, enable auto-merge, or push directly to `main` or `develop`.
- Preserve required CI, administrator enforcement, conversation resolution, force-push/deletion prevention and the exact-candidate physical release gate. Sole-maintainer approving reviews are intentionally zero; this does not authorize automatic merging.
- Do not register a daily-use computer as a public self-hosted runner or execute contributor PR code on a physical lab machine.
- Keep credentials, device identifiers, activation/Gowin licenses, raw NVS/corpus/flash backups and private audit logs out of Git and public CI artifacts.
- Use English for project documentation. Keep historical hardware evidence tied to its original candidate and distinguish MCU framebuffer/queued controls from independent physical/battery qualification.
