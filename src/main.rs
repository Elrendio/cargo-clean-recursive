use std::env::{args, current_dir};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use anyhow::{Context, Result};
use clap::{App, Arg};

fn main() {
	if let Err(e) = _main() {
		eprintln!("Error: {}", e);
		for c in e.chain().skip(1) {
			eprintln!("	< {}", c);
		}
		exit(1);
	}
}

fn _main() -> Result<()> {
	let mut args: Vec<String> = args().collect();
	if args.len() >= 2 && &args[1] == "clean-recursive" {
		args.remove(1);
	}

	let matches = App::new("cargo clean-recursive")
		.bin_name("cargo clean-recursive")
		.arg(Arg::with_name("doc").short("d").long("doc").help("Deletes documents"))
		.arg(
			Arg::with_name("release")
				.short("r")
				.long("release")
				.help("Deletes release target"),
		)
		.arg(
			Arg::with_name("depth")
				.long("depth")
				.default_value("64")
				.help("Recursive serarch depth limit"),
		)
		.arg(Arg::with_name("path").short("p").long("path").help("Target directory"))
		.arg(
			Arg::with_name("exclude_dirs")
				.short("ed")
				.long("exclude_dirs")
				.help("Exclude directories"),
		)
		.get_matches_from(&args);

	let del_mode = match (matches.is_present("doc"), matches.is_present("release")) {
		(false, false) => DeleteMode::All,
		(doc, release) => DeleteMode::Partial { doc, release },
	};

	let depth_str = matches.value_of("depth").expect("'depth' should be exists");
	let depth: usize = depth_str
		.parse()
		.with_context(|| format!("parsing '{}' as number", depth_str))?;

	let path = if let Some(path) = matches.value_of("path") {
		PathBuf::from(path)
	} else {
		current_dir().context("getting current_dir")?
	};

	let exclude_dirs = if let Some(exclude_dirs) = matches.value_of("exclude_dirs") {
		exclude_dirs.split(' ').collect::<Vec<_>>()
	} else {
		Default::default()
	};

	process_dir(Path::new(&path), depth, &Config { exclude_dirs, del_mode })?;

	Ok(())
}

struct Config<'s> {
	exclude_dirs: Vec<&'s str>,
	del_mode: DeleteMode,
}

#[derive(Debug)]
enum DeleteMode {
	All,
	Partial { doc: bool, release: bool },
}

fn process_dir(path: &Path, depth: usize, config: &Config) -> Result<()> {
	if depth == 0 {
		return Ok(());
	}

	detect_and_clean(path, &config.del_mode).with_context(|| format!("cleaning directory {:?}", path))?;

	for e in path
		.read_dir()
		.with_context(|| format!("reading directory {:?}", path.canonicalize()))?
	{
		let e = e?;
		if e.file_type()?.is_dir()
			&& config
				.exclude_dirs
				.iter()
				.find(|&&d| e.file_name().as_os_str().to_str().map_or(false, |e| e.ends_with(d)))
				.is_none()
		{
			if let Err(e) = process_dir(&e.path(), depth - 1, config) {
				eprintln!("Warn: {}", e);
				for c in e.chain().skip(1) {
					eprintln!("	at: {}", c);
				}
			}
		}
	}

	Ok(())
}

fn detect_and_clean(path: &Path, del_mode: &DeleteMode) -> Result<()> {
	if !path.join("Cargo.toml").exists() {
		return Ok(());
	}

	let target_dir = path.join("target");
	if !target_dir.exists() || !target_dir.is_dir() {
		return Ok(());
	}

	eprintln!("Cleaning {:?}", path);

	match del_mode {
		DeleteMode::All => {
			Command::new("cargo").args(&["clean"]).current_dir(path).output()?;
		}
		DeleteMode::Partial { doc, release } => {
			if *doc {
				Command::new("cargo")
					.args(&["clean", "--doc"])
					.current_dir(path)
					.output()?;
			}
			if *release {
				Command::new("cargo")
					.args(&["clean", "--release"])
					.current_dir(path)
					.output()?;
			}
		}
	}
	Ok(())
}
