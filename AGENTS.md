# Agent Operating Manual

> **NOTICE TO AI AGENTS**: Read this file **completely** before performing any operations in this repository. Strict adherence to these protocols is mandatory.

## 1. Project Overview
**hale** is a lightweight, high-precision network connection quality monitor written in Rust.
- **Core Stack**: Rust (2021), `tokio` (Async I/O), `ratatui` (TUI), `clap` (CLI).
- **Architecture**:
    - `Monitor`: Concurrent TCP probing logic.
    - `Analysis`: Statistical aggregation and majority voting.
    - `UI`: Rendering logic.
    - `Utils`: Logging and helpers.

## 2. Git & Workflow Protocol (CRITICAL)

### 2.1 Worktree Mandate
**NEVER** work directly in the main directory for features or fixes.
- **Requirement**: You must create a new git worktree for every task.
- **Command Pattern**:
  ```bash
  git worktree add worktrees/<branch-name> -b <branch-name>
  ```

### 2.2 Branch Naming
- **Rule**: NO SLASHES (`/`) in branch names. This prevents directory nesting issues in `worktrees/`.
- **Allowed**: `feat-new-widget`, `fix-memory-leak`.
- **Forbidden**: `feat/new-widget`, `fix/memory-leak`.

### 2.3 Commit Standards
- **Standard**: Conventional Commits (strictly enforced for `release-plz`).
- **Structure**: `<type>: <description>`
- **Types**:
    - `feat`: New feature (minor version bump).
    - `fix`: Bug fix (patch version bump).
    - `chore`: Maintenance, docs, refactoring (no version bump).
    - `BREAKING CHANGE`: Triggers major version bump.

### 2.4 Protected Branches
- **Rule**: NEVER push directly to `main`.
- **Rule**: ALL changes must go through a Pull Request.

## 3. Development Lifecycle

### Phase 1: Setup
1.  Read `AGENTS.md`.
2.  **Roadmap Check**: Check `docs/roadmap.md`. If the current task is missing, add it as a new line item (`- [ ] Task Name`).
3.  Create a unique branch (no slashes) and worktree.
4.  Switch context to the worktree directory.

### Phase 2: Implementation
1.  **Tests First**: If fixing a bug, reproduce it with a test. If adding a feature, plan the test.
2.  **Code**: Implement changes adhering to `clippy` and `fmt` standards.
3.  **Verify**:
    ```bash
    cargo test
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    ```

### Phase 3: Review & Merge
1.  Push branch to origin.
2.  Create Pull Request.
3.  **Verification Loop**:
    - Poll CI status every **3 minutes**.
    - **Do not proceed** until PR is merged to `main`.

### Phase 4: Post-Merge & Cleanup
1.  After merge, checkout `main` and pull latest.
2.  Run `cargo test` on `main` to verify stability.
3.  **STOP**: Ask user for explicit confirmation to cleanup.
4.  Upon confirmation:
    1.  **Update Roadmap**: Mark the task as completed in `docs/roadmap.md` (`- [x] Task Name`).
    2.  **Docs Cleanup**: Delete temporary files in `docs/` (e.g., PRDs, plans), **keeping** `docs/roadmap.md`.
    3.  Remove worktree and branch:
        ```bash
        git worktree remove worktrees/<branch-name>
        git branch -d <branch-name>
        ```

## 4. Technical Constraints
- **Async Runtime**: Use `tokio`. Avoid blocking threads.
- **Error Handling**: Use `Result<T, E>`. **NO `unwrap()`** in production code.
- **Platform**: Cross-platform compatibility required (Linux, macOS, Windows).
