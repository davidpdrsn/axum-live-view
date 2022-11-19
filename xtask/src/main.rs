#![allow(clippy::try_err)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{
    env, fs,
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
        Opt::Ts(Ts::Install(opt)) => ts_install(opt)?,
        Opt::Ts(Ts::Build(opt)) => ts_build(opt)?,
        Opt::Ts(Ts::Precompile(opt)) => ts_precompile(opt)?,
        Opt::Codegen => codegen()?,
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
enum Opt {
    Ts(Ts),
    Codegen,
}

/// Typescript related commands
#[derive(Debug, StructOpt)]
enum Ts {
    Install(TsInstall),
    Build(TsBuild),
    Precompile(TsPrecompile),
}

/// Build the typescript assets
#[derive(Debug, StructOpt)]
struct TsBuild {
    /// Only perform type checking, don't emit any files
    #[structopt(short, long)]
    check: bool,
}

fn ts_build(opt: TsBuild) -> Result {
    let TsBuild { check } = opt;

    let mut cmd = Command::new("npx");
    cmd.arg("tsc");
    cmd.current_dir(project_root().join("assets"));

    if check {
        cmd.arg("--noEmit");
    } else {
        cmd.args(&["--build"]);
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

    #[structopt(long)]
    no_install: bool,
}

fn ts_precompile(opt: TsPrecompile) -> Result {
    let TsPrecompile { check, no_install } = opt;

    if !no_install {
        ts_install(TsInstall {})?;
    }

    ts_build(TsBuild { check: false })?;

    let precompiled = project_root().join("assets-precompiled");

    let code_before_build = fs::read_to_string(precompiled.join("axum_live_view.min.js"))?;

    let mut install_cmd = Command::new("npm");
    install_cmd.arg("install");
    install_cmd.current_dir(&precompiled);
    run_cmd(install_cmd)?;

    let mut build_command = Command::new("npx");
    build_command.arg("webpack");
    build_command.current_dir(&precompiled);
    run_cmd(build_command)?;

    let code = fs::read_to_string(precompiled.join("axum_live_view.min.js"))?;

    if check {
        if code_before_build != code {
            Err("Code on disk doesn't match")?;
        }
    } else {
        fs::write(
            precompiled.join("axum_live_view.min.js.gz"),
            gzip(code.as_bytes())?,
        )?;

        fs::write(
            precompiled.join("axum_live_view.hash.txt"),
            calculate_hash(&code).to_string(),
        )?;
    }

    Ok(())
}

fn calculate_hash<T: std::hash::Hash>(t: &T) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn gzip(input: &[u8]) -> Result<Vec<u8>> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::prelude::*;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(input)?;
    Ok(encoder.finish()?)
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
                    quote! { #ty: LiveView, }
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
                        );
                    )*
                };

                let update = {
                    let match_arms = types.iter().map(|ty| {
                        quote! {
                            #either_name::#ty(msg) => {
                                let Self { views: (#(#types,)*), render } = self;
                                let Updated {
                                    live_view: #ty,
                                    js_commands,
                                    spawns,
                                } = #ty.update(msg, data);
                                let spawns = spawns
                                    .into_iter()
                                    .map(|future| {
                                        Box::pin(async move {
                                            #either_name::#ty(future.await)
                                        }) as _
                                    })
                                    .collect::<Vec<_>>();
                                Updated {
                                    live_view: Self {
                                        views: (#(#types,)*),
                                        render,
                                    },
                                    js_commands,
                                    spawns,
                                }
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
                    impl<F, #(#types,)*> LiveView for Combine<(#(#types,)*), F>
                    where
                        #(#live_view_bounds)*
                        F: Fn( #(#fn_bound_args,)* ) -> Html<#either> + Send + Sync + 'static,
                    {
                        type Message = #either;

                        fn mount(
                            &mut self,
                            uri: Uri,
                            request_headers: &HeaderMap,
                            handle: ViewHandle<Self::Message>,
                        ) {
                            #mount
                        }

                        fn update(
                            mut self,
                            msg: Self::Message,
                            data: Option<EventData>,
                        ) -> Updated<Self> {
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
        use axum::http::{HeaderMap, Uri};
        use serde::{Deserialize, Serialize};

        #[allow(missing_debug_implementations)]
        pub struct Combine<V, F> {
            pub(super) views: V,
            pub(super) render: F,
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
