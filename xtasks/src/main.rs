use std::path::PathBuf;

use cargo_metadata::{MetadataCommand, TargetKind};
use clap::{Arg, Command, builder::styling::{AnsiColor, Style}};
use color_print::{cformat};

fn find_subcommands() -> Vec<(String, PathBuf, Option<PathBuf>)> {
    let mut subcommands = Vec::new();
    
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_dir = PathBuf::from(manifest_dir);

        for entry in walkdir::WalkDir::new(manifest_dir.clone()).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_dir() {
                continue;
            }

            if entry.path() == manifest_dir {
                continue;
            }

            let path = entry.path().join("Cargo.toml");
            if path.exists() {
                if let Ok(metadata) = MetadataCommand::new().manifest_path(path.clone()).no_deps().exec() {
                    if let Some(package) = metadata.root_package() {
                        let has_binary = package.targets.iter().any(|target| {
                            target.kind.iter().any(|k| matches!(k, TargetKind::Bin))
                        });

                        if !has_binary {
                            continue;
                        }

                        let config = entry.path().join(".cargo").join("config.toml");

                        if config.exists() {
                            subcommands.push((package.name.to_string(), path, Some(config)));
                        } else {
                            subcommands.push((package.name.to_string(), path, None));
                        }
                    }
                }
            }
        }
    }

    subcommands
}

/// Runs the specified manifest from the current working directory but uses its cargo config if provided.
fn run_from_cwd(
    manifest_path: &PathBuf,
    config_path: &Option<PathBuf>,
    cargo_args: &[String],
    sub_args: &[String],
) -> Result<std::process::ExitStatus, ()> {
    let mut command = std::process::Command::new("cargo");

    // Run manifest with the correct config.
    if let Some(config) = config_path {
        command.arg("--config").arg(config);
    }

    command.arg("run");

    // Forward arbitrary cargo run flags provided before the subcommand.
    if !cargo_args.is_empty() {
        command.args(cargo_args);
    }

    command.arg("--manifest-path").arg(manifest_path);

    if !sub_args.is_empty() {
        command.arg("--");
        command.args(sub_args);
    }

    command.status().map_err(|_| ())
}

fn main() {
    logging::init();

    let subcommands = find_subcommands();

    let mut root = Command::new("xtask")
        .disable_help_subcommand(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .about("Helper tool for running subtasks")
        .override_usage("xtask [OPTIONS] [CARGO_OPTIONS] <COMMAND> <ARGS>")
        .arg(
            Arg::new("cargo-opts")
                .help_heading("Cargo Options")
                .value_name("option")
                .help("Forwarded to `cargo run`")
                .num_args(0..)
                .allow_hyphen_values(true)
                .trailing_var_arg(true),
        )
        .arg(
            Arg::new("sub-args")
                .help_heading("Command Arguments")
                .value_name("arg")
                .help("Arguments passed to <COMMAND>")
                .num_args(0..)
                .allow_hyphen_values(true)
                .trailing_var_arg(true),
        );

    for (name, _, _) in &subcommands {
        // I am sorry.
        let leaked_subc: &'static str = Box::leak(name.clone().into_boxed_str());

        let sc = Command::new(leaked_subc)
        // Idea: pass through all args after the subcommand to the underlying command
            .disable_help_flag(true)
            .arg(
                Arg::new("args")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true),
            );

        root = root.subcommand(sc);
    }

    let usage = root.render_usage();
    // everything before a known subcommand is for cargo, everything after belongs to the subcommand.
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        let _ = root.print_help();
        println!();
        return;
    }

    // Find the first token that matches a known subcommand name.
    let sub_idx = args.iter().position(|a| subcommands.iter().any(|(name, _, _)| name == a));

    let (cargo_args, cmd_name, sub_args) = match sub_idx {
        Some(idx) => {
            if args[0..idx].iter().any(|a| a == "--help" || a == "-h") {
                let _ = root.print_help();
                println!();
                return;
            }

            let (cargo_args, rest) = args.split_at(idx);
            if rest.is_empty() {
                let _ = root.print_help();
                println!();
                return;
            }
            let cmd_name = &rest[0];
            let sub_args = &rest[1..];
            (cargo_args.to_vec(), cmd_name.clone(), sub_args.to_vec())
        }
        None => {
            // No known subcommand found.
            let style = Style::new().fg_color(Some(AnsiColor::White.into()));
            log::error!("{style:#}{}", cformat!("unknown subcommand '<b><yellow>{}</yellow></b>'\n\n{}\n\nFor more information, try '<s>--help</s>'.", args.get(0).cloned().unwrap_or_default(), usage.ansi()));
            std::process::exit(1);
        }
    };

    if let Some((_, manifest_path, config_path)) = subcommands.iter().find(|(n, _, _)| n == &cmd_name) {
        match run_from_cwd(manifest_path, config_path, &cargo_args, &sub_args) {
            Ok(status) => {
                std::process::exit(status.code().unwrap_or(1));
            },
            Err(()) => {
                log::error!("failed to execute subcommand '{cmd_name}'");
                std::process::exit(1);
            }
        }
    } else {
        let style = Style::new().fg_color(Some(AnsiColor::White.into()));
        log::error!("{style:#}{}", cformat!("unknown subcommand '<b><yellow>{}</yellow></b>'\n\n{}\n\nFor more information, try '<s>--help</s>'.", cmd_name, usage.ansi()));
        std::process::exit(1);
    }
}
