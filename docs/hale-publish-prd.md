# PRD: Hale Automated Crate Publishing

## 1. Overview
The goal is to automate the publishing of the `hale` crate to crates.io whenever a "significant change" occurs. This streamlines the release process, ensures consistency, and reduces manual errors. The automation will be handled via GitHub Actions.

## 2. Requirements

### 2.1 Functional Requirements
- **Triggering**: The system must automatically detect when a release is warranted.
- **Versioning**: The system must determine the correct semantic version bump (Major, Minor, or Patch) based on the nature of the changes.
- **Validation**: The system must ensure all tests pass before publishing.
- **Publishing**: The system must publish the crate to crates.io using the provided secret `CRATES_IO_TOKEN`.
- **Notification/Feedback**: The system should indicate success or failure of the publication.

### 2.2 Key Decisions: Determining "Significant Change"
To answer "how the action decides if the version should be bumped", we propose using **Conventional Commits**.

*   **Mechanism**: The CI pipeline analyzes commit messages since the last release.
    *   `fix: ...` -> Patch bump (0.1.0 -> 0.1.1)
    *   `feat: ...` -> Minor bump (0.1.0 -> 0.2.0)
    *   `BREAKING CHANGE: ...` -> Major bump (0.1.0 -> 1.0.0)
*   If no such commits exist, no release is triggered.

### 2.3 User Stories
- As a maintainer, I want to push a commit with message `feat: add new monitor` and have the CI automatically bump the version and publish to crates.io.
- As a maintainer, I want to ensure that broken code is never published to crates.io.

## 3. Proposed Workflow (GitHub Actions)

We recommend using **Release-plz** or a similar automated release PR flow.

**Option A: Release PR (Recommended)**
1.  **Detection**: A scheduled action (or on push to main) checks for new conventional commits.
2.  **Proposal**: It creates a "Release PR" that updates `Cargo.toml`, `CHANGELOG.md`, and creates a git tag.
3.  **Approval**: The user reviews and merges the Release PR.
4.  **Publishing**: On merge (push to main with version change), the action publishes to crates.io.

**Option B: Direct Publish on Push**
1.  **Detection**: On every push to `main`.
2.  **Action**: If commits warrant a release, it bumps version, commits back to main, and publishes.
    *   *Risk*: Can be messy if builds fail after bump.

**Recommendation**: **Option B: Direct Publish on Push**. The user has selected the fully automated approach.

**Selected Workflow:**
1.  **Trigger**: Push to `main`.
2.  **Analysis**: Analyze commits using Conventional Commits.
3.  **Action**: If a release is required:
    *   Bump version in `Cargo.toml`.
    *   Generate/Update `CHANGELOG.md`.
    *   Commit and Tag (e.g., `v0.2.0`).
    *   Push changes to repo.
    *   Publish to crates.io.

## 4. Safety Mechanisms
- **CI Checks**: Run `cargo test` and `cargo clippy` before any publish step.
- **Dry Run**: Verify `cargo publish --dry-run` succeeds.
- **Token Security**: Use `secrets.CRATES_IO_TOKEN` only in the publish step.

## 5. Next Steps
1.  Design the Technical Spec for the Release PR workflow.
2.  Implement the GH Action.
