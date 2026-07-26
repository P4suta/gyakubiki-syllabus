//! Minimal binary shim; all implementation stays private in `syllabus-cli`.

fn main() -> std::process::ExitCode {
    syllabus_cli::run()
}
