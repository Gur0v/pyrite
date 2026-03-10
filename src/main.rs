use clap::Parser;
use std::io::{self, Write};
use std::process::{Command, ExitCode};

#[derive(Parser)]
#[command(name = "pyrite", disable_help_flag = true)]
struct Cli {
    #[arg(short = 'S', long)] sync_mode:   bool,
    #[arg(short = 'R', long)] remove_mode: bool,
    #[arg(short = 'Q', long)] query_mode:  bool,
    #[arg(short = 'V', long)] version:     bool,
    #[arg(short = 's', long)] search:      bool,
    #[arg(short = 'y', action = clap::ArgAction::Count)] refresh: u8,
    #[arg(short = 'u', long)] upgrade:     bool,
    #[arg(short = 'd', action = clap::ArgAction::Count)] nodeps: u8,
    #[arg(short = 'p', long)] pretend:     bool,
    #[arg(short = 'h', long)] help:        bool,
    #[arg(long)]              noconfirm:   bool,
    #[arg(long)]              moo:         bool,
    packages: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let is_root = unsafe { libc::getuid() == 0 };

    if args.len() == 1 {
        return run("eselect news read", false, is_root);
    }

    let cli = Cli::try_parse_from(&args).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    dispatch(cli, is_root)
}

fn dispatch(cli: Cli, is_root: bool) -> ExitCode {
    if cli.help    { print_help(); return ExitCode::SUCCESS; }
    if cli.version { print_version(); return ExitCode::SUCCESS; }
    if cli.moo     { return run("emerge --moo", false, is_root); }

    match (cli.sync_mode, cli.remove_mode, cli.query_mode) {
        (true, true, _) => die("Cannot use -S and -R simultaneously."),
        (true, _, _)    => handle_sync(&cli, is_root),
        (_, true, _)    => handle_remove(&cli, is_root),
        (_, _, true)    => handle_query(&cli, is_root),
        _               => die("No valid mode selected. See --help for usage."),
    }
}

fn handle_sync(cli: &Cli, is_root: bool) -> ExitCode {
    let idle = cli.refresh == 0 && !cli.upgrade && cli.packages.is_empty() && !cli.search;
    if idle {
        return die("No action for sync mode. Use -Sy, -Syu, or -S <pkg>.");
    }

    if cli.search {
        let pkgs = require_packages(&cli.packages, "Search requires a query.");
        return run(&format!("eix {pkgs}"), false, is_root);
    }

    if cli.refresh > 0 {
        let sync_cmd = if cli.refresh >= 2 { "eix-sync -a" } else { "eix-sync" };
        let code = run(sync_cmd, cli.pretend, is_root);
        if code != ExitCode::SUCCESS { return code; }
    }

    if cli.upgrade {
        let flags = emerge_flags(cli, "uDN");
        let code = run(&format!("emerge {flags} @world"), cli.pretend, is_root);
        if code != ExitCode::SUCCESS { return code; }
        if !cli.pretend && !cli.noconfirm { post_update_hooks(is_root); }
    }

    if !cli.packages.is_empty() {
        let flags = emerge_flags(cli, "");
        return run(&format!("emerge {flags} {}", cli.packages.join(" ")), cli.pretend, is_root);
    }

    ExitCode::SUCCESS
}

fn handle_remove(cli: &Cli, is_root: bool) -> ExitCode {
    let pkgs = require_packages(&cli.packages, "No packages specified for removal.");
    let flags = emerge_flags(cli, "");

    let cmd = if cli.nodeps >= 2 {
        format!("emerge {flags}C {pkgs}")
    } else {
        format!("emerge {flags}cv {pkgs}")
    };

    run(&cmd, cli.pretend, is_root)
}

fn handle_query(cli: &Cli, is_root: bool) -> ExitCode {
    if cli.search {
        let pkgs = require_packages(&cli.packages, "Search requires a query.");
        return run(&format!("eix --installed {pkgs}"), false, is_root);
    }
    die("No action for query mode. Use -Qs <pkg>.")
}

fn emerge_flags(cli: &Cli, extra: &str) -> String {
    if cli.noconfirm {
        let p = if cli.pretend { " -p" } else { "" };
        format!("--ask=n -v{extra}{p}")
    } else {
        let p = if cli.pretend { "p" } else { "" };
        format!("-av{extra}{p}")
    }
}

fn require_packages(pkgs: &[String], msg: &str) -> String {
    if pkgs.is_empty() { die(msg); }
    pkgs.join(" ")
}

fn run(cmd: &str, pretend: bool, is_root: bool) -> ExitCode {
    let needs_root = ["emerge", "eix-sync", "cfg-update", "eclean", "eselect news"]
        .iter()
        .any(|p| cmd.starts_with(p));

    if pretend {
        println!("\x1b[1;34m(pretend)\x1b[0m would run: {cmd}");
        return ExitCode::SUCCESS;
    }

    let final_cmd = if needs_root && !is_root {
        format!("sudo {cmd}")
    } else {
        cmd.to_string()
    };

    let status = Command::new("sh")
        .args(["-c", &final_cmd])
        .status()
        .unwrap_or_else(|e| { eprintln!("failed to spawn: {e}"); std::process::exit(1) });

    if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(status.code().unwrap_or(1) as u8)
    }
}

fn die(msg: &str) -> ExitCode {
    eprintln!("\x1b[1;31merror:\x1b[0m {msg}");
    ExitCode::FAILURE
}

fn post_update_hooks(is_root: bool) {
    print!("\n>> Update complete. Run maintenance? (cfg-update, eclean) [y/N]: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if input.trim().eq_ignore_ascii_case("y") {
        run("cfg-update -a", false, is_root);
        run("eclean-dist -d", false, is_root);
    }
}

fn print_version() {
    let portage = Command::new("emerge")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Portage version unknown".into());
    println!("pyrite v{} - {portage}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
"Usage: pyrite <operation> [options] [targets]

Operations:
  -S, --sync          Synchronize packages (emerge/eix-sync)
  -R, --remove        Remove packages from the system (depclean/unmerge)
  -Q, --query         Query the local package database (eix --installed)

Sync Options (-S):
  -y, --refresh       Download fresh package databases (eix-sync)
                      Pass twice (-yy) to force refresh (eix-sync -a)
  -u, --sysupgrade    Upgrade installed packages (-avuDN @world)
  -s, --search        Search remote repositories for matching strings
  -p, --pretend       Display what would be done without executing

Remove Options (-R):
  -d, --nodeps        Skip dependency checks. Pass twice (-Rdd) to force
                      unmerge (emerge -C) which is highly dangerous.

Query Options (-Q):
  -s, --search        Search locally installed packages for matching strings

General Options:
  --noconfirm         Do not ask for confirmation (emerge --ask=n)
  --moo               Display the Portage mascot
  -h, --help          Display detailed help and exit

Pyrite provides a familiar interface for managing Gentoo systems using
Arch-like syntax, wrapping emerge, eix, and eselect."
    );
}
