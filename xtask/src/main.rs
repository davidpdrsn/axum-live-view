#![allow(clippy::try_err)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{
    env, fs,
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
        Opt::Ts(Ts::Precompile(opt)) => ts_precompile(opt)?,
        Opt::Examples(Examples::Build(opt)) => examples_build(opt)?,
        Opt::Examples(Examples::Run(opt)) => examples_run(opt)?,
        Opt::Codegen => codegen()?,
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
enum Opt {
    Ts(Ts),
    Examples(Examples),
    Codegen,
}

/// Typescript related commands
#[derive(Debug, StructOpt)]
enum Ts {
    Build(TsBuild),
    Install(TsInstall),
    Precompile(TsPrecompile),
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

/// Precompile JavaScript
#[derive(Debug, StructOpt)]
struct TsPrecompile {
    #[structopt(long)]
    check: bool,
}

fn ts_precompile(opt: TsPrecompile) -> Result {
    let TsPrecompile { check } = opt;

    examples_build(ExamplesBuild {
        skip_ts: false,
        skip_deps: false,
        which: Some("counter".to_owned()),
    })?;

    let code = fs::read_to_string(project_root().join("examples/counter/dist/bundle.js"))?;

    let path = project_root().join("assets/axum_live_view.min.js");

    if check {
        let current = fs::read_to_string(path)?;
        if current != code {
            Err("Code on disk doesn't match")?;
        }
    } else {
        fs::write(path, code)?;
    }

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
    #[structopt(long)]
    release: bool,

    #[structopt(flatten)]
    build: ExamplesBuild,
}

fn examples_run(opt: ExamplesRun) -> Result {
    let ExamplesRun { release, build } = opt;

    let which = if let Some(which) = build.which.as_ref() {
        which.to_owned()
    } else {
        Err("Must specify which example to run")?
    };

    examples_build(build)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_root());
    cmd.args(&["run", "-p", &format!("example-{}", which)]);
    if release {
        cmd.arg("--release");
    }
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

fn codegen() -> Result {
    const N: usize = 8;

    fn eithers() -> TokenStream {
        (1..=N)
            .map(|n| {
                let name = format_ident!("Either{}", n);

                let types = (1..=n).map(|n| format_ident!("T{}", n)).collect::<Vec<_>>();

                let variants = types.iter().map(|name| {
                    quote! { #name(#name), }
                });

                quote! {
                    #[allow(unreachable_pub, missing_debug_implementations)]
                    #[derive(PartialEq, Serialize, Deserialize)]
                    pub enum #name <#(#types,)*> {
                        #(#variants)*
                    }
                }
            })
            .collect()
    }

    fn live_view_impls() -> TokenStream {
        (1..=N)
            .map(|n| {
                let types = (1..=n).map(|n| format_ident!("T{}", n)).collect::<Vec<_>>();

                let live_view_bounds = types.iter().map(|ty| {
                    quote! { #ty: LiveView<Error = E>, }
                });

                let either_name = format_ident!("Either{}", n);

                let either = {
                    let variants = types.iter().map(|ty| {
                        quote! { #ty::Message }
                    });
                    quote! {
                        #either_name<#(#variants,)*>
                    }
                };

                let fn_bound_args = types.iter().map(|_| {
                    quote! { Html<#either> }
                });

                let mount = quote! {
                    let Self { views: (#(#types,)*), .. } = self;
                    #(
                        #types.mount(
                            uri.clone(),
                            request_headers,
                            handle.clone().with(#either_name::#types),
                        ).await?;
                    )*
                };

                let update = {
                    let match_arms = types.iter().map(|ty| {
                        quote! {
                            #either_name::#ty(msg) => {
                                let Self { views: (#(#types,)*), render } = self;
                                let (#ty, cmds) = #ty.update(msg, data).await?.into_parts();
                                Ok(Updated::new(Self {
                                    views: (#(#types,)*),
                                    render,
                                })
                                .with_all(cmds))
                            }
                        }
                    });

                    quote! {
                        match msg {
                            #( #match_arms )*
                        }
                    }
                };

                let render = quote! {
                    let Self { views: (#(#types,)*), render } = self;
                    render( #( #types.render().map(#either_name::#types), )* )
                };

                quote! {
                    #[allow(non_snake_case)]
                    #[async_trait]
                    impl<F, E, #(#types,)*> LiveView for Combine<(#(#types,)*), F>
                    where
                        #(#live_view_bounds)*
                        F: Fn( #(#fn_bound_args,)* ) -> Html<#either> + Send + Sync + 'static,
                        E: std::fmt::Display + Send + Sync + 'static,
                    {
                        type Message = #either;
                        type Error = E;

                        async fn mount(
                            &mut self,
                            uri: Uri,
                            request_headers: &HeaderMap,
                            handle: ViewHandle<Self::Message>,
                        ) -> Result<(), Self::Error> {
                            #mount
                            Ok(())
                        }

                        async fn update(
                            mut self,
                            msg: Self::Message,
                            data: Option<EventData>,
                        ) -> Result<Updated<Self>, Self::Error> {
                            #update
                        }

                        fn render(&self) -> Html<Self::Message> {
                            #render
                        }
                    }
                }
            })
            .collect()
    }

    let eithers = eithers();
    let live_view_impls = live_view_impls();
    let code = quote! {
        use crate::{
            event_data::EventData,
            html::Html,
            live_view::{Updated, ViewHandle},
            LiveView,
        };
        use async_trait::async_trait;
        use axum::http::{HeaderMap, Uri};
        use serde::{Deserialize, Serialize};

        pub fn combine<V, F>(views: V, render: F) -> Combine<V, F> {
            Combine { views, render }
        }

        #[allow(missing_debug_implementations)]
        pub struct Combine<V, F> {
            views: V,
            render: F,
        }

        #eithers
        #live_view_impls
    };

    let combine_mod_path = project_root().join("axum-live-view/src/live_view/combine.rs");

    fs::write(combine_mod_path, code.to_string())?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_root());
    cmd.args(&["fmt"]);
    cmd.status()?;

    Ok(())
}
