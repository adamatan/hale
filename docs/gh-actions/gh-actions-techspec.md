# Technical Specification: GitHub Actions CI Workflow for 'hale'

## Workflow Overview
- **File Path**: `.github/workflows/ci.yml`
- **Workflow Name**: `CI`
- **Runner**: `ubuntu-latest`

## Triggers
The workflow will be triggered on:
- **Push**: To any branch.
- **Pull Request**: Against any branch.

```yaml
on:
  push:
    branches: ["**"]
  pull_request:
    branches: ["**"]
```

## Job Definition: `test`
The CI process consists of a single job named `test`.

### Actions Used
- `actions/checkout@v4`: To pull the repository code.
- `dtolnay/rust-toolchain@stable`: To set up the stable Rust toolchain (Edition 2021 compliant).
- `swatinem/rust-cache@v2`: To handle efficient caching of Rust build artifacts and dependencies.

### Step-by-Step Logic
1.  **Checkout**: Uses `actions/checkout@v4`.
2.  **Toolchain Setup**: Uses `dtolnay/rust-toolchain@stable`.
3.  **Cache**: Uses `swatinem/rust-cache@v2`.
4.  **Build**: Runs `cargo build --verbose`.
5.  **Test**: Runs `cargo test --verbose`.

## YAML Implementation Preview
```yaml
name: CI

on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v4
    - name: Install stable toolchain
      uses: dtolnay/rust-toolchain@stable
    - name: Rust Cache
      uses: swatinem/rust-cache@v2
    - name: Build
      run: cargo build --verbose
    - name: Run tests
      run: cargo test --verbose
```
*Note: The `branches` configuration will be adjusted to `**` or omitted to match the "all branches" requirement as per the PRD.*

## Badge Implementation
To be added to the top of `README.md`:

```markdown
[![CI](https://github.com/adam-matan/hale/actions/workflows/ci.yml/badge.svg)](https://github.com/adam-matan/hale/actions/workflows/ci.yml)
```

## Considerations
- **Branch Scope**: By not specifying branches in the `on` block (or using `**`), GitHub Actions defaults to all branches for pushes and pull requests.
- **Efficiency**: `swatinem/rust-cache` is preferred as it automatically handles the complex caching logic for Rust projects.
- **Rust Edition**: The `dtolnay/rust-toolchain@stable` will provide the latest stable compiler which supports Edition 2021.
