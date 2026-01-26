# PRD: GitHub Actions CI Workflow for 'hale'

## Goals
- Automate the build and test process for the 'hale' repository.
- Ensure code quality and stability across all branches.
- Reduce manual verification effort for contributors.
- Provide clear visibility of the build status via a README badge.

## User Stories
- As a developer, I want my code to be automatically tested upon pushing to any branch, so I can catch regressions early.
- As a maintainer, I want to see the build status of pull requests, so I can merge changes with confidence.
- As a user, I want to see a build status badge in the README, so I know if the current version is stable.

## Acceptance Criteria
- A GitHub Actions workflow is triggered on every `push` to any branch.
- A GitHub Actions workflow is triggered on every `pull_request`.
- The workflow successfully installs the Rust toolchain (Edition 2021).
- The workflow runs `cargo test` and passes.
- Dependency caching is implemented to speed up subsequent builds.
- A GitHub Actions status badge is correctly displayed in the `README.md` file.

## Functional Requirements
- **Environment**: Ubuntu Latest.
- **Rust Setup**: Use a reliable action to set up the Rust 2021 toolchain (e.g., `dtolnay/rust-toolchain`).
- **Triggers**: 
  - `push` to all branches.
  - `pull_request` to all branches.
- **Caching**: Implement caching for `target/` and `~/.cargo/` to optimize build times.
- **README Status Badge**: 
  - Add a Markdown badge to the top of `README.md`.
  - The badge must link to the Actions tab and show the status of the primary CI workflow.
- **Workflow Steps**:
  1. **Checkout**: Pull the latest code from the repository.
  2. **Toolchain Setup**: Install the stable Rust toolchain.
  3. **Cache**: Restore/Save cargo registry and build artifacts.
  4. **Build**: Execute `cargo build --verbose`.
  5. **Test**: Execute `cargo test --verbose`.
