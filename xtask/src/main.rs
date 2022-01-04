#![allow(clippy::try_err)]

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
};
use structopt::StructOpt;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

macro_rules! ensure {
    ($cond:expr, $err:literal) => { ensure!($cond, $err,) };

    ($cond:expr, $err:literal, $($arg:tt)*) => {
        if !$cond {
            Err(format!($err, $($arg)*))?;
        }
    };
}

fn main() -> Result {
    let opt = Opt::from_args();

    match opt {
        Opt::Ts(Ts::Build(opt)) => ts_build(opt)?,
        Opt::Ts(Ts::Install(opt)) => ts_install(opt)?,
        Opt::Examples(Examples::Build(opt)) => examples_build(opt)?,
        Opt::Examples(Examples::Run(opt)) => examples_run(opt)?,
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
enum Opt {
    Ts(Ts),
    Examples(Examples),
}

/// Typescript related commands
#[derive(Debug, StructOpt)]
enum Ts {
    Build(TsBuild),
    Install(TsInstall),
}

/// Build the typescript assets
#[derive(Debug, StructOpt)]
struct TsBuild {
    /// Watch files for changes
    #[structopt(short, long)]
    watch: bool,

    /// Clean compiled files and artifacts
    #[structopt(long)]
    clean: bool,

    /// Only perform type checking, don't emit any files
    #[structopt(short, long, conflicts_with_all = &["clean"])]
    check: bool,
}

fn ts_build(opt: TsBuild) -> Result {
    let TsBuild {
        watch,
        clean,
        check,
    } = opt;

    let mut cmd = Command::new("npx");
    cmd.current_dir(project_root().join("assets"));

    cmd.arg("tsc");

    if watch {
        cmd.arg("-w");
    }

    if clean {
        cmd.args(&["--build", "--clean"]);
    }

    if check {
        cmd.arg("--noEmit");
    }

    run_cmd(cmd)?;

    Ok(())
}

/// Install typescript dependecies
#[derive(Debug, StructOpt)]
struct TsInstall {}

fn ts_install(opt: TsInstall) -> Result {
    let TsInstall {} = opt;

    let mut cmd = Command::new("npm");
    cmd.current_dir(project_root().join("assets"));
    cmd.arg("install");

    run_cmd(cmd)?;

    Ok(())
}

/// Typescript related commands
#[derive(Debug, StructOpt)]
enum Examples {
    Build(ExamplesBuild),
    Run(ExamplesRun),
}

#[derive(Debug, StructOpt)]
struct ExamplesBuild {
    #[structopt(long)]
    skip_ts: bool,

    #[structopt(long)]
    skip_deps: bool,

    #[structopt()]
    which: Option<String>,
}

fn examples_build(opt: ExamplesBuild) -> Result {
    let ExamplesBuild {
        which,
        skip_ts,
        skip_deps,
    } = opt;

    if !skip_ts {
        println!("building typescript");
        ts_build(TsBuild {
            watch: false,
            clean: false,
            check: false,
        })?;
    }

    let example_dirs = read_dir(project_root().join("examples"))?
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .filter(|path| {
            if let Some(which) = &which {
                path.ends_with(which)
            } else {
                true
            }
        });

    let mut threads = Vec::new();
    for dir in example_dirs {
        threads.push(std::thread::spawn(move || {
            let contains_webpack_config = read_dir(&dir)?
                .into_iter()
                .filter_map(|entry| entry.ok())
                .any(|entry| {
                    let path = entry.path();
                    let name = path.file_name().and_then(|name| name.to_str());
                    name == Some("webpack.config.js")
                });

            if contains_webpack_config {
                if !skip_deps {
                    println!("installing dependencies {}", dir.display());
                    let mut install_cmd = Command::new("npm");
                    install_cmd.arg("install");
                    install_cmd.current_dir(&dir);
                    run_cmd(install_cmd)?;
                }

                println!("building {}", dir.display());
                let mut cmd = Command::new("npx");
                cmd.arg("webpack");
                cmd.current_dir(&dir);
                run_cmd(cmd)?;
            }

            Result::Ok(())
        }));
    }

    for t in threads {
        t.join().expect("thread panicked")?;
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
struct ExamplesRun {
    #[structopt(flatten)]
    build: ExamplesBuild,
}

fn examples_run(opt: ExamplesRun) -> Result {
    let which = if let Some(which) = opt.build.which.as_ref() {
        which.to_owned()
    } else {
        Err("Must specify which example to run")?
    };

    examples_build(opt.build)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_root());
    cmd.args(&["run", "-p", &format!("example-{}", which)]);
    cmd.env(
        "RUST_LOG",
        format!("axum_live_view=trace,example_{}=trace", which),
    );
    cmd.status()?;

    Ok(())
}

fn run_cmd(mut cmd: Command) -> Result {
    let desc = format!("{:?}", cmd);
    let status = cmd.status()?;
    ensure!(status.success(), "`{}` failed", desc);
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
