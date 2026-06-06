# Git Workflow

Remote:

```powershell
git remote add origin git@github.com:Pippinrao/rust-can.git
```

## Branch Model

rust-can uses a `feature/bugfix -> dev -> master` workflow.

| Branch | Purpose | Merge target |
| --- | --- | --- |
| `master` | Stable release branch. Only reviewed release-ready changes from `dev` land here. | None |
| `dev` | Integration branch for completed feature and bugfix branches. | `master` |
| `feature/<short-name>` | New capability, compatibility work, architecture improvements, or benchmarks. | `dev` |
| `bugfix/<short-name>` | Correctness, compatibility, warning, coverage, or performance regression fixes. | `dev` |

## Rules

- Start new implementation work from `dev`.
- Use `feature/<short-name>` for planned work and `bugfix/<short-name>` for defects.
- Merge feature and bugfix branches into `dev` only after tests and documentation are updated.
- Merge `dev` into `master` only for release-ready snapshots.
- Do not commit generated build output, external checkouts, extracted log corpora, or generated data fixtures.
- Keep performance claims tied to benchmark files under `benchmarks/results/YYYY-MM-DD/`.

## Common Commands

```powershell
git switch dev
git pull --ff-only origin dev
git switch -c feature/<short-name>

cargo test --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80

git push -u origin feature/<short-name>
```

After review:

```powershell
git switch dev
git merge --ff-only feature/<short-name>
git push origin dev
```

For releases:

```powershell
git switch master
git merge --ff-only dev
git push origin master
```
