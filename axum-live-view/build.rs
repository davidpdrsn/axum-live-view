use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::{env, fs, path::Path};

const N: usize = 8;

fn main() {
    let eithers = eithers();
    let live_view_impls = live_view_impls();
    let code = quote! {
        #eithers
        #live_view_impls
    };

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("combine_impls.rs");
    fs::write(&dest_path, code.to_string()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}

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
