// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8PathBuf;
use eyre::eyre;

use crate::environment;

fn parse_command(command: &[String], output: Utf8PathBuf) -> eyre::Result<std::process::Command> {
    if command.is_empty() {
        return Err(eyre!(
            "invalid `serve.command`: command list cannot be empty"
        ));
    }

    let mut serve = std::process::Command::new(&command[0]);
    for arg in &command[1..] {
        if arg == "<output>" {
            serve.arg(&output);
            continue;
        }
        serve.arg(arg);
    }
    Ok(serve)
}

pub(super) fn spawn_serve_process() -> eyre::Result<std::process::Child> {
    let command = environment::serve_command();
    let mut serve = parse_command(&command, environment::output_dir())?
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(serve_stdout) = serve.stdout.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => println!("[serve] {line}"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stdout read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    if let Some(serve_stderr) = serve.stderr.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => color_print::ceprintln!("<r>[serve] Error: {line}</>"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stderr read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    Ok(serve)
}
