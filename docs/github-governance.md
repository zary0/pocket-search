# GitHub governance and public activation

## Maintainer and review policy

The initial maintainer is [@zary0](https://github.com/zary0). [CODEOWNERS](https://github.com/zary0/pocket-search/blob/develop/.github/CODEOWNERS) assigns review responsibility, including CI/release infrastructure, licensing, and security policy. Accepted contributions and support limits are in [CONTRIBUTING.md](../CONTRIBUTING.md).

A CODEOWNERS file alone does not require an approval. GitHub must enforce required owner reviews. Authors cannot approve their own PRs: add an independent maintainer with write access to the appropriate CODEOWNERS entries before enabling this rule for maintainer-authored changes. Do not count automated review as the required human approval or add an administrator bypass to resolve that staffing gap.

## Current verified status

As checked on **2026-10-10**, `zary0/pocket-search` is **private**, with `develop` as its default branch. Reading branch protection returns HTTP 403 with GitHub's message requiring GitHub Pro or a public repository. No branch-protection rule is currently enforced. This is a limitation of the current private setup, not a limitation of public repositories on Free.

| Control | Status |
| --- | --- |
| Branch PR/CI/owner-review requirements, force-push/deletion prevention | Prepared payloads; not enabled because of the current private-repository restriction |
| Weekly Cargo and GitHub Actions version updates | Configuration targets `develop`; becomes active from the default branch |
| Dependabot alerts and security-update PRs | Enabled and read back via GitHub API; security updates are not paused |
| Repository secret scanning and push protection | Not enabled in the current private setup; enable and verify on public activation |
| Private vulnerability reporting | Not enabled/verified; existing confidential-contact fallback in SECURITY.md remains in force |
| Physical release gate | Workflow/evidence validation exists; full native application and lab E2E remain pending |

GitHub supports protected branches in public repositories on **GitHub Free**, including Free organizations; private branches require a supported paid plan. CODEOWNERS is also available for Free public repositories. See [protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) and [code owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners). Do not retain the private-plan limitation as a claim after publication.

## Branch rules to enforce

| Setting | `develop` | `main` |
| --- | --- | --- |
| Changes through PRs | Required | Required |
| Approving human reviews / owner review | One / required | One / required |
| Dismiss approvals after changed commits | Enabled | Enabled |
| Required checks, from GitHub Actions app ID 15368 | `ci`, `native-firmware` | `ci`, `native-firmware`, `qualify / chromatic-required` |
| Up to date with base / conversations resolved | Required | Required |
| Force pushes / branch deletion | Disabled | Disabled |
| Apply rules to administrators | Enabled | Enabled |

The exact check names and app ID were observed on workflow runs. `main` promotion must carry the passing physical result for the exact head, as described in [release policy](releases.md). Missing physical checks remain blocking; do not remove them to obtain a green merge. No qualified firmware release exists yet.

The committed payloads are [develop](https://github.com/zary0/pocket-search/blob/develop/.github/branch-protection.develop.json) and [main](https://github.com/zary0/pocket-search/blob/develop/.github/branch-protection.main.json). They are reviewable configuration, not proof that GitHub has applied it.

## Activation after public publication or a supported plan

Changing visibility and buying a plan are separate owner decisions. Before activation, confirm the independent review owner, inspect any existing rules, and compare these payloads with them. The PUT operation replaces the named branch's protection configuration; preserve any stronger existing controls rather than blindly replacing them.

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
