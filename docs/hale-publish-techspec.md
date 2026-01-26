# Technical Specification: Direct Publish Workflow

## 1. Architecture Overview

The "Direct Publish" workflow is a GitHub Action designed to automate the release process of the `hale` crate. It operates on a "Push to Main" trigger, effectively implementing a Continuous Deployment (CD) pipeline for the Rust crate.

### Data Flow
1.  **Developer** pushes code to `main`.
2.  **GitHub Action** triggers.
3.  **Validation**: Code is checked (formatted, linted, tested).
4.  **Analysis**: Commit history is analyzed for Conventional Commits.
5.  **Decision**:
    *   If changes warrant a release -> Version is bumped, Changelog updated.
    *   If no changes -> Workflow terminates.
6.  **Mutation**: New version and changelog are committed and pushed back to `main`.
7.  **Publication**: The crate is published to crates.io and a Git tag is pushed.

## 2. Selected Tooling & Rationale

**Selected Tool: `release-plz` (CLI)**

We have evaluated `release-plz`, `cargo-release`, and `cocogitto`.

*   **Decision**: Use `release-plz` (CLI binary).
*   **Rationale**:
    *   **Automation Focus**: Unlike `cargo-release` which is interactive-heavy, `release-plz` is designed for CI automation.
    *   **All-in-One**: It handles Conventional Commits analysis, Semantic Versioning bumping, Changelog generation (`git-cliff` integration), and Publishing.
    *   **Rust Ecosystem**: It is a native Rust tool, recommended in the PRD.
    *   **Direct Mode**: While often used with PRs, the CLI `update` and `release` commands support the direct manipulation required for the "Direct Publish" workflow.

## 3. Detailed Workflow Steps

The implementation will be a single GitHub Action workflow file (e.g., `.github/workflows/release.yml`).

### Workflow Triggers
```yaml
on:
  push:
    branches:
      - main
```

### Job: `release-plz`

**Permissions**:
*   `contents: write` (Required to push commits and tags)
*   `id-token: write` (Optional, if using OIDC, but we are using Token)

**Environment**:
*   `CARGO_REGISTRY_TOKEN`: `${{ secrets.CRATES_IO_TOKEN }}`

**Steps**:

1.  **Checkout Code**
    *   Action: `actions/checkout@v4`
    *   Config: `fetch-depth: 0` (Critical for analyzing commit history)
    *   Config: `token: ${{ secrets.GITHUB_TOKEN }}`

2.  **Install Rust Toolchain**
    *   Action: `dtolnay/rust-toolchain@stable`

3.  **Install `release-plz`**
    *   Command: `cargo install release-plz` (or download pre-built binary for speed)

4.  **Run CI Checks (Safety Mechanism)**
    *   Command: `cargo test`
    *   Command: `cargo clippy -- -D warnings`
    *   *Note*: These must pass before proceeding.

5.  **Configure Git User**
    *   Identity for the automated commit.
    *   Name: `github-actions[bot]`
    *   Email: `41898282+github-actions[bot]@users.noreply.github.com`

6.  **Run Release Logic**
    *   This step orchestrates the bump and publish.
    *   **Command**: `release-plz release --project-manifest Cargo.toml`
    *   *Wait, Correction*: The standard `release-plz release` command publishes *existing* versions and pushes tags. It does not bump.
    *   *Revised Logic for Direct Publish*:
        1.  `release-plz update` (Bumps version, updates Changelog locally).
        2.  Check for git changes.
        3.  If changes exist:
            *   Commit changes: `git commit -am "chore: release v..."`
            *   Push to main: `git push`
            *   Run `release-plz release` (Publishes to crates.io, creates and pushes git tag).

### YAML Structure Plan

```yaml
jobs:
  release:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      
      - uses: dtolnay/rust-toolchain@stable

      # Optimization: Cache cargo or use pre-built binary
      - name: Install release-plz
        run: curl -L -o release-plz.tar.gz https://github.com/MarcoIeni/release-plz/releases/latest/download/release-plz-x86_64-unknown-linux-gnu.tar.gz && tar xf release-plz.tar.gz && sudo mv release-plz /usr/local/bin/

      - name: CI Checks
        run: |
          cargo test
          cargo clippy -- -D warnings

      - name: Git Config
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

      - name: Release Flow
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CRATES_IO_TOKEN }}
        run: |
          # 1. Update Manifest and Changelog
          release-plz update
          
          # 2. Check if anything changed
          if [[ -n $(git status -s) ]]; then
            echo "Changes detected. Preparing release..."
            
            # 3. Commit and Push
            git add .
            git commit -m "chore: release"
            git push origin main
            
            # 4. Publish to Crates.io and Tag
            release-plz release
          else
            echo "No release needed."
          fi
```

## 4. Handling "No Release Needed"

The logic relies on `release-plz update`.
*   If no Conventional Commits (fix/feat/breaking) are found since the last tag, `release-plz update` will not modify `Cargo.toml`.
*   The shell condition `if [[ -n $(git status -s) ]]` detects this state.
*   If no changes, the script prints "No release needed" and exits successfully without performing git operations or publishing.

## 5. Handling Git User Identity

To ensure commits are attributed correctly to the bot and don't look like they came from an arbitrary user:
*   **Name**: `github-actions[bot]`
*   **Email**: `41898282+github-actions[bot]@users.noreply.github.com`
This is the standard GitHub Actions bot identity.

## 6. Prevention of Infinite CI Loops

The critical risk in "Direct Publish" is the "Push to Main" trigger firing again when the Action pushes its own commit.

*   **Solution**: GitHub Actions default behavior.
*   **Mechanism**: When using the standard `${{ secrets.GITHUB_TOKEN }}` to authenticate git operations, GitHub intentionally **does not trigger** new workflow runs from events pushed by that token.
*   **Verification**: Ensure we are using the default `actions/checkout` token or explicitly passing `GITHUB_TOKEN`. We are **not** using a Personal Access Token (PAT), which *would* cause an infinite loop.

## 7. Security & Permissions

*   **`secrets.CRATES_IO_TOKEN`**: This secret must be added to the GitHub Repository secrets. It is exposed *only* to the release job.
*   **`permissions: contents: write`**: Explicitly granted to allow the Action to push commits (version bumps) and tags to the repository.
