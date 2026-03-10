use clap::Parser;
use std::process::Command;
use std::io::{self, Write};

#[derive(Parser)]
#[command(name = "pyrite", disable_help_flag = true)]
struct Cli {
    #[arg(short = 'S', long)]
    sync_mode: bool,

    #[arg(short = 'R', long)]
    remove_mode: bool,

    #[arg(short = 'Q', long)]
    query_mode: bool,

    #[arg(short = 'V', long)]
    version: bool,

    #[arg(short = 's', long)]
    search: bool,

    #[arg(short = 'y', action = clap::ArgAction::Count)]
    refresh: u8,

    #[arg(short = 'u', long)]
    upgrade: bool,

    #[arg(short = 'd', action = clap::ArgAction::Count)]
    nodeps: u8,

    #[arg(short = 'p', long)]
    pretend: bool,

    #[arg(short = 'h', long)]
    help: bool,

    #[arg(long)]
    noconfirm: bool,

    #[arg(long)]
    moo: bool,

    packages: Vec<String>,
}

fn print_detailed_help() {
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

Examples:
  pyrite              Read unread Gentoo news items
  pyrite -Syu         Sync repositories and upgrade the entire system
  pyrite -S <pkg>     Install a specific package
  pyrite -Ss <query>  Search for a package in the tree
  pyrite -R <pkg>     Safely remove a package (depclean)
  pyrite -Rdd <pkg>   Forcefully remove a package (unmerge)

Pyrite provides a familiar interface for managing Gentoo systems using 
Arch-like syntax, wrapping emerge, eix, and eselect."
    );
}

fn get_portage_version() -> String {
    Command::new("emerge")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Portage version unknown".to_string())
}

fn main() {
    let raw_args = std::env::args().collect::<Vec<String>>();
    let is_root = unsafe { libc::getuid() == 0 };

    if raw_args.len() == 1 {
        execute("eselect news read", false, is_root);
        return;
    }

    let cli = match Cli::try_parse_from(raw_args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if cli.help {
        print_detailed_help();
        return;
    }

    if cli.version {
        println!("pyrite v{} - {}", env!("CARGO_PKG_VERSION"), get_portage_version());
        return;
    }

    if cli.moo {
        execute("emerge --moo", false, is_root);
        return;
    }

    if cli.sync_mode && cli.remove_mode {
        error_exit("Cannot use sync (-S) and remove (-R) simultaneously.");
    }

    if cli.sync_mode {
        if cli.refresh == 0 && !cli.upgrade && cli.packages.is_empty() && !cli.search {
            error_exit("No action specified for sync mode. Use -Sy, -Syu, or -S <pkg>.");
        }

        if cli.search {
            if cli.packages.is_empty() { error_exit("Search requires a query."); }
            execute(&format!("eix -I {}", cli.packages.join(" ")), false, is_root);
            return;
        }

        if cli.refresh > 0 {
            execute(if cli.refresh >= 2 { "eix-sync -a" } else { "eix-sync" }, cli.pretend, is_root);
        }

        if cli.upgrade {
            let mut opts = if cli.noconfirm { "--ask=n".to_string() } else { "a".to_string() };
            if cli.pretend { opts.push('p'); }
            execute(&format!("emerge -v{}uDN @world", opts), cli.pretend, is_root);
            if !cli.pretend && !cli.noconfirm { post_update_hooks(is_root); }
        }

        if !cli.packages.is_empty() {
            let mut opts = if cli.noconfirm { "--ask=n".to_string() } else { "a".to_string() };
            if cli.pretend { opts.push('p'); }
            execute(&format!("emerge -v{} {}", opts, cli.packages.join(" ")), cli.pretend, is_root);
        }
    } else if cli.remove_mode {
        if cli.packages.is_empty() {
            error_exit("No packages specified for removal.");
        }
        let mut opts = if cli.noconfirm { "--ask=n".to_string() } else { "a".to_string() };
        if cli.pretend { opts.push('p'); }
        
        let rm_cmd = if cli.nodeps >= 2 {
            format!("emerge -v{}C {}", opts, cli.packages.join(" "))
        } else {
            format!("emerge -v{}cv {}", opts, cli.packages.join(" "))
        };
        execute(&rm_cmd, cli.pretend, is_root);
    } else if cli.query_mode && cli.search {
        if cli.packages.is_empty() { error_exit("Search requires a query."); }
        execute(&format!("eix -I --installed {}", cli.packages.join(" ")), false, is_root);
    } else {
        error_exit("No valid mode selected. See --help for usage.");
    }
}

fn error_exit(msg: &str) -> ! {
    eprintln!("\x1b[1;31merror:\x1b[0m {}", msg);
    std::process::exit(1);
}

fn post_update_hooks(is_root: bool) {
    print!("\n>> Update complete. Run maintenance? (cfg-update, eclean) [y/N]: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if input.trim().to_lowercase() == "y" {
        execute("cfg-update -a", false, is_root);
        execute("eclean-dist -d", false, is_root);
    }
}

fn execute(cmd: &str, pretend: bool, is_root: bool) {
    let bin = cmd.split_whitespace().next().unwrap_or("");
    if Command::new("which").arg(bin).output().map(|o| !o.status.success()).unwrap_or(true) {
        error_exit(&format!("Command '{}' not found. Please install it.", bin));
    }

    if pretend && !cmd.contains("emerge") {
        println!("(Pretend) Would execute: {}", cmd);
        return;
    }

    let needs_root = cmd.contains("emerge") || cmd.contains("sync") || cmd.contains("update") || cmd.contains("eclean") || cmd.contains("news");
    let final_cmd = if needs_root && !is_root {
        format!("sudo {}", cmd)
    } else {
        cmd.to_string()
    };

    let status = Command::new("sh")
        .arg("-c")
        .arg(&final_cmd)
        .status()
        .unwrap_or_else(|_| std::process::exit(1));

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
