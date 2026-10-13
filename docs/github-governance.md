# GitHub governance and public activation

## Maintainer and review policy

The initial maintainer is [@zary0](https://github.com/zary0). [CODEOWNERS](https://github.com/zary0/pocket-search/blob/develop/.github/CODEOWNERS) assigns review responsibility, including CI/release infrastructure, licensing, and security policy. Accepted contributions and support limits are in [CONTRIBUTING.md](../CONTRIBUTING.md).

A CODEOWNERS file records responsibility but does not itself enforce approval.
During sole-maintainer operation, `main` and `develop` require PRs and passing CI,
with approving reviews set to **zero** and mandatory CODEOWNER approval disabled.
The maintainer performs merges manually; agents and CI do not merge PRs or enable
auto-merge. Add an independent reviewer before requiring one approving owner
review, because authors cannot approve their own PRs. Do not add an administrator
bypass to solve that staffing gap.

This manual-merge policy is not human identity attestation: a holder of the
maintainer's credentials can invoke GitHub APIs. Repository auto-merge is disabled,
workflow tokens are read-only and cannot approve PRs, and [agent rules](../AGENTS.md)
prohibit merging. Keep owner credentials out of workflows and public artifacts.

## Pre-publication audit and activation record

The 2026-10-13 audit covers all fetched branch/tag/PR histories and the 84 source
commits referenced by the 107 existing CI runs, plus all 107 available current run-log
archives plus the one earlier rerun attempt, 30 artifacts and published Issue/review content. Reports and raw logs
are retained privately outside Git. Candidate matches are reviewed rather than
silently ignored. See the [sanitized pre-publication record](evidence/public-readiness/prepublication.json).

The pre-publication record is a dated snapshot, not a claim that future settings
or repository contents remain unchanged. Before public activation, apply and
read back the controls below. After activation, verify both branch protections,
secret scanning, push protection and the private report form. Configuration files
are not proof that GitHub enforces them. Physical release qualification remains
blocked independently of source publication.

GitHub supports protected branches in public repositories on **GitHub Free**,
including Free organizations; the earlier private-plan restriction does not apply
after publication. See [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches).

## Branch rules to enforce

| Setting | `develop` | `main` |
| --- | --- | --- |
| Changes through PRs | Required | Required |
| Approving reviews / mandatory owner review while sole-maintained | Zero / disabled | Zero / disabled |
| Dismiss approvals after changed commits | Enabled | Enabled |
| Required checks, from GitHub Actions app ID 15368 | `ci`, `native-firmware`, `secret-scan` | `ci`, `native-firmware`, `secret-scan`, `qualify / chromatic-required` |
| Up to date with base / conversations resolved | Required | Required |
| Force pushes / branch deletion | Disabled | Disabled |
| Apply rules to administrators | Enabled | Enabled |

The exact check names and app ID were observed on workflow runs. `main` promotion must carry the passing physical result for the exact head, as described in [release policy](releases.md). Missing physical checks remain blocking; do not remove them to obtain a green merge. No qualified firmware release exists yet.

The committed payloads are [develop](https://github.com/zary0/pocket-search/blob/develop/.github/branch-protection.develop.json) and [main](https://github.com/zary0/pocket-search/blob/develop/.github/branch-protection.main.json). They are reviewable configuration, not proof that GitHub has applied it.

## Activation after public publication or a supported plan

Public activation is an owner-authorized publishing action. Inspect any existing rules and compare these sole-maintainer payloads before activation. The PUT operation replaces the named branch's protection configuration; preserve any stronger existing controls rather than blindly replacing them.

From the source checkout, apply the reviewed payloads:

```sh
gh api --method PUT repos/zary0/pocket-search/branches/develop/protection --input .github/branch-protection.develop.json
gh api --method PUT repos/zary0/pocket-search/branches/main/protection --input .github/branch-protection.main.json
```

Read both back and confirm every required setting, including the check app IDs and administrator enforcement:

```sh
gh api repos/zary0/pocket-search/branches/develop/protection
gh api repos/zary0/pocket-search/branches/main/protection
gh api repos/zary0/pocket-search/codeowners/errors
```

If the repository remains private on a paid plan, check eligibility for each security feature separately; branch-protection availability does not establish that scanning/reporting features are available.

In repository **Settings → Security and quality**, enable/confirm Dependabot alerts and security updates, repository secret scanning, repository push protection, and private vulnerability reporting. Public repositories have free secret-scanning coverage; explicitly verify repository-level controls rather than relying on a contributor's account-level push protection. See [secret scanning](https://docs.github.com/en/code-security/how-tos/secure-your-secrets/detect-secret-leaks/enable-secret-scanning) and [push protection](https://docs.github.com/en/code-security/concepts/secret-security/about-push-protection).

Verify the repository's `security_and_analysis` settings and that **Security → Report a vulnerability** appears. Then update [SECURITY.md](../SECURITY.md), the README, and this status table to the verified state and preferred reporting link. Never describe planned controls as enabled. Do not publish test secrets to check detection.

Stable `v*` tag immutability and the `chromatic-lab` environment's trusted-ref restrictions are additional release prerequisites in [release policy](releases.md) and [lab setup](hardware-e2e.md); branch rules do not automatically protect tags or environments.

## Dependency maintenance

Dependabot checks Cargo and pinned GitHub Actions weekly and targets `develop`, matching the default branch used for security-update PRs. Updates are reviewed rather than auto-merged. Review changed behavior, minimum toolchain compatibility, upstream action SHA provenance, and licenses; refresh [third-party notices](../THIRD_PARTY_NOTICES.md) and copied license/copyright texts for changed packages. A dependency update does not waive CI or physical release qualification.

## Maintainer-only merging and immutable version tags

The separate [update ruleset](../.github/ruleset.manual-merges.json) restricts
updates to `main` and `develop` to the named maintainer user, through PRs only.
No GitHub App or workflow actor is granted that bypass. This bypass covers only
the update rule in this separate ruleset; the classic branch protections still
require CI and conversation resolution and apply to administrators. It is not a
bypass of failed checks. Owner credentials remain capable of acting as the owner,
so agents must follow the manual-merge rule and workflows receive no such token.

The [stable-tag ruleset](../.github/ruleset.version-tags.json) prohibits updates
and deletions of `v*` tags without bypass actors. Creating a new annotated tag
still requires the source/version/main and physical release checks. Neither
ruleset publishes a firmware release.

## Ongoing secret checks

The `secret-scan` check uses checksum-pinned official Gitleaks 8.30.1, full available checkout history, default provider/generic rules, archive decoding and fully redacted diagnostics. The only source exception requires both the exact SDK configuration path and the exact upstream public default for disabled OpenThread. It does not ignore that entire file, source hashes, or arbitrary credentials. Native repository secret scanning and push protection are enabled separately; CI scanning does not replace them.

## Verified public activation on 2026-10-13

After the maintainer manually merged PR #18, the repository became Public. Both branch protections, maintainer-only PR updates, immutable version tags, secret scanning, push protection and private vulnerability reporting were applied and read back. Anonymous access and merged CI checks passed. The final audit covered 114 current run-log archives, one prior attempt and 33 artifacts, with no verified credentials in that scope. See the [activation snapshot](evidence/public-readiness/activation.json); it does not guarantee future settings or qualify a hardware release.

Personal-repository protection payloads omit organization-only review bypass allowances. No review bypass is granted.
