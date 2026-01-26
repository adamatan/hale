# Implementation Plan - Automated Crate Publishing

This plan details the steps to implement the automated release workflow for the `hale` crate, as defined in `docs/hale-publish-prd.md` and `docs/hale-publish-techspec.md`.

## 1. Implementation Overview
- **Goal**: Automate version bumping, changelog generation, and publishing to crates.io on every push to `main` that contains significant changes.
- **Tooling**: GitHub Actions, `release-plz`.
- **Trigger**: Push to `main` branch.

## 2. Implementation Phases

### Phase 1: Preparation
Ensure the environment is ready for the automated workflow.

- [ ] **Verify Secrets**:
    - Ensure `CRATES_IO_TOKEN` is set in the repository secrets.
    - *Action for User*: Check Settings > Secrets and variables > Actions.

### Phase 2: Workflow Creation
Create the GitHub Actions workflow file.

- [ ] **Create `.github/workflows/release.yml`**:
    - **Trigger**: `push` to `main`.
    - **Permissions**: `contents: write`.
    - **Steps**:
        1. Checkout code (fetch-depth: 0).
        2. Install Rust toolchain.
        3. Install `release-plz`.
        4. Run CI checks (`cargo test`, `cargo clippy`).
        5. Configure Git user (`github-actions[bot]`).
        6. Run release logic:
            - `release-plz update` (updates local files).
            - Check for changes.
            - If changed: Commit, Push, and `release-plz release`.

### Phase 3: Documentation
Update project documentation to reflect the new release process.

- [ ] **Update `README.md`**:
    - Add a section explaining the automated release process.
    - Mention usage of Conventional Commits.

### Phase 4: Validation
Verify the implementation.

- [ ] **Syntax Check**: Verify YAML syntax of the workflow file.
- [ ] **Dry Run (Optional)**: Can be simulated by pushing a non-release commit and checking logs.

## 3. Detailed Task Breakdown

### Task 1: Create Release Workflow
**File**: `.github/workflows/release.yml`
**Content**:
```yaml
name: Release

on:
  push:
    branches:
      - main

permissions:
  contents: write

jobs:
  release:
    name: Release
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install release-plz
        uses: listendev/action-release-plz@v0.5
        # Alternatively, install via cargo as specified in Tech Spec if preferred, 
        # but the official action or pre-built binary is faster. 
        # Tech spec suggested: curl download or cargo install.
        # Let's stick to the Tech Spec recommendation of manual install for control.
        
      - name: Install release-plz (Manual)
        run: |
          curl -L -o release-plz.tar.gz https://github.com/MarcoIeni/release-plz/releases/latest/download/release-plz-x86_64-unknown-linux-gnu.tar.gz
          tar xf release-plz.tar.gz
          sudo mv release-plz /usr/local/bin/

      - name: Run CI Checks
        run: |
          cargo test
          cargo clippy -- -D warnings

      - name: Git Config
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "41898282+github-actions[bot]@users.noreply.github.com"

      - name: Release
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
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

### Task 2: Update Documentation
**File**: `README.md`
**Content to Append**:
```markdown
## Release Process

This project uses an automated release workflow powered by [release-plz](https://github.com/MarcoIeni/release-plz).

*   **Trigger**: Pushing to the `main` branch.
*   **Versioning**: Determined automatically based on [Conventional Commits](https://www.conventionalcommits.org/).
    *   `fix:` -> Patch bump.
    *   `feat:` -> Minor bump.
    *   `BREAKING CHANGE:` -> Major bump.
*   **Publishing**: Automatically publishes to [crates.io](https://crates.io/crates/hale) and creates a GitHub release/tag.
```

## 4. Testing Strategy
- **Unit Tests**: Covered by `cargo test` in the workflow.
- **Integration**: The workflow itself is an integration test.
- **Verification**:
    1.  Push the workflow file.
    2.  Observe the Action run on GitHub.
    3.  It should succeed and print "No release needed" (assuming no pending conventional commits).
