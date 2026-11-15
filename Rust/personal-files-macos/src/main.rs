#[cfg(target_os = "macos")]
mod native;

fn main() {
    let raw = std::env::args_os().skip(1).collect::<Vec<_>>();
    if raw.first().is_some_and(|s| s == "--install-bundle") {
        if raw.len() != 3 {
            eprintln!("Usage: personal-files-macos --install-bundle STAGED_APP DESTINATION");
            std::process::exit(2);
        }
        if let Err(e) = personal_files::bundle::publish(
            std::path::Path::new(&raw[1]),
            std::path::Path::new(&raw[2]),
        ) {
            eprintln!("Installation not published; existing app unchanged: {e}");
            std::process::exit(1);
        }
        return;
    }
    let mut args = std::env::args().skip(1);
    let first = args.next();
    if args.next().is_some() {
        eprintln!("Usage: personal-files-macos [PATH | --self-test | --help]");
        std::process::exit(2);
    }
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        println!("Personal Files — native Rust macOS browser\nUsage: personal-files-macos [PATH]\n--self-test runs only synthetic desktop fixtures.");
        return;
    }
    let fixture = first.as_deref() == Some("--self-test");
    if !fixture && first.as_ref().is_some_and(|s| s.starts_with('-')) {
        eprintln!("Unknown option. Use --help or an absolute path.");
        std::process::exit(2);
    }
    let start = if fixture {
        None
    } else {
        first.map(|s| {
            personal_files::core::location(
                &s,
                &std::env::current_dir().unwrap_or_else(|_| personal_files::core::home()),
            )
            .unwrap()
        })
    };
    #[cfg(target_os = "macos")]
    if let Err(e) = native::run(start, fixture) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (start, fixture);
        eprintln!("This application requires macOS.");
        std::process::exit(1);
    }
}
