# Security tooling policy

EduTalent production security validation uses only:

- RustSec `cargo-audit` for Rust dependency advisories;
- repository-owned static production-configuration policy checks;
- first-party GitHub Actions only for checkout and artifact transport in the security job.

The production security job must not delegate scanning to an additional third-party action. Any proposed scanner change requires an explicit security/tooling review before it can replace this approved execution path.
