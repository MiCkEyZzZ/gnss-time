# Pull Request

Please include:

- **Scope**: what area of the codebase this touches (e.g. `gnss-time`).
- **Summary**: brief description of the change.
- **Testing**: how did you verify it works? (unit tests, manual steps).
- **Checklist**:
  - [ ] `cargo fmt --check`
  - [ ] `taplo format`
  - [ ] `cargo clippy -- -D warnings`
  - [ ] `cargo test`
  - [ ] New tests added / existing tests updated
  - [ ] Added the necessary rustdoc comments.
  - [ ] Changelog updated if applicable
