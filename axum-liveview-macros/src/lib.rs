#![warn(
    clippy::all,
    // clippy::dbg_macro,
    clippy::todo,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::mem_forget,
    clippy::unused_self,
    clippy::filter_map_next,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::mismatched_target_os,
    clippy::await_holding_lock,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::fn_params_excessive_bools,
    clippy::exit,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::verbose_file_reads,
    clippy::unnested_or_patterns,
    clippy::str_to_string,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    // missing_debug_implementations,
    // missing_docs
)]
#![deny(unreachable_pub, private_in_public)]
#![allow(elided_lifetimes_in_paths, clippy::type_complexity)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, allow(clippy::float_cmp))]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::fmt::Write;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Block, Ident, LitStr, Token,
};

#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tree = match syn::parse::<Tree>(input) {
        Ok(tree) => tree,
        Err(err) => return err.into_compile_error().into(),
    };

    let tokens = tree.into_token_stream();

    // useful for debugging:
    // println!("{}", tokens);

    tokens.into()
}

#[derive(Debug, Clone)]
struct Tree {
    nodes: Vec<HtmlNode>,
}

impl Parse for Tree {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let nodes = parse_many(input)?;
        Ok(Self { nodes })
    }
}

fn parse_many<P>(input: ParseStream) -> syn::Result<Vec<P>>
where
    P: Parse,
{
    let mut nodes = Vec::new();
    while !input.is_empty() {
        nodes.push(input.parse()?);
    }
    Ok(nodes)
}

#[derive(Debug, Clone)]
enum HtmlNode {
    Doctype(Doctype),
    TagNode(TagNode),
    LitStr(LitStr),
    Block(Block),
    If(If<Tree>),
    For(For),
    Match(Match),
}

impl Parse for HtmlNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![<]) && input.peek2(Token![!]) {
            input.parse().map(Self::Doctype)
        } else if input.peek(Token![<]) && !input.peek2(Token![/]) {
            input.parse().map(Self::TagNode)
        } else if input.peek(LitStr) {
            input.parse().map(Self::LitStr)
        } else if let Ok(block) = input.parse() {
            Ok(Self::Block(block))
        } else if input.peek(Token![if]) {
            input.parse().map(Self::If)
        } else if input.peek(Token![for]) {
            input.parse().map(Self::For)
        } else if input.peek(Token![match]) {
            input.parse().map(Self::Match)
        } else {
            let span = input.span();
            Err(syn::Error::new(span, "Unexpected token"))
        }
    }
}

#[derive(Debug, Clone)]
struct Doctype;

impl Parse for Doctype {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        mod kw {
            syn::custom_keyword!(DOCTYPE);
            syn::custom_keyword!(html);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![!]>()?;
        input.parse::<kw::DOCTYPE>()?;
        input.parse::<kw::html>()?;
        input.parse::<Token![>]>()?;

        Ok(Self)
    }
}

#[derive(Debug, Clone)]
struct TagNode {
    open: Ident,
    attrs: Vec<Attr>,
    close: Option<TagClose>,
}

#[derive(Debug, Clone)]
struct TagClose {
    inner: Vec<HtmlNode>,
    close: Close,
}

impl Parse for TagNode {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![<]>()?;
        let open = input.parse()?;

        let mut attrs = Vec::new();
        while input.fork().parse::<AttrIdent>().is_ok() {
            attrs.push(input.parse()?);
        }

        if input.parse::<Token![/]>().is_ok() {
            input.parse::<Token![>]>()?;
            return Ok(Self {
                open,
                attrs,
                close: None,
            });
        }

        input.parse::<Token![>]>()?;

        let next_is_close = || input.peek(Token![<]) && input.peek2(Token![/]);
        let mut inner = Vec::new();
        while !next_is_close() {
            inner.push(input.parse::<HtmlNode>()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let close = input.parse::<Close>()?;
        input.parse::<Token![>]>()?;

        if open != close.0 {
            let span = open
                .span()
                .join(close.0.span())
                .unwrap_or_else(|| close.0.span());
            return Err(syn::Error::new(span, "Unmatched close tag"));
        }

        Ok(Self {
            open,
            attrs,
            close: Some(TagClose { inner, close }),
        })
    }
}

#[derive(Debug, Clone)]
struct Attr {
    ident: AttrIdent,
    value: AttrValue,
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<AttrIdent>()?;

        let value = if input.parse::<Token![=]>().is_ok() {
            input.parse()?
        } else {
            AttrValue::Unit(Unit)
        };

        Ok(Self { ident, value })
    }
}

#[derive(Debug, Clone)]
struct AttrIdent {
    ident: String,
}

impl Parse for AttrIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = if input.parse::<Token![type]>().is_ok() {
            "type".to_owned()
        } else if input.parse::<Token![for]>().is_ok() {
            "for".to_owned()
        } else {
            let idents = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(input)?;
            let mut out = String::new();
            let mut iter = idents.iter().peekable();
            while let Some(ident) = iter.next() {
                if iter.peek().is_some() {
                    let _ = write!(out, "{}-", ident);
                } else {
                    let _ = write!(out, "{}", ident);
                }
            }
            out
        };

        Ok(Self { ident })
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum AttrValue {
    LitStr(LitStr),
    Block(Block),
    Unit(Unit),
    If(If<Box<AttrValue>>),
    None,
}

impl Parse for AttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lh = input.lookahead1();

        if lh.peek(syn::token::Brace) {
            input.parse().map(AttrValue::Block)
        } else if lh.peek(syn::token::Paren) {
            input.parse().map(AttrValue::Unit)
        } else if lh.peek(syn::LitStr) {
            input.parse().map(AttrValue::LitStr)
        } else if lh.peek(Token![if]) {
            input.parse().map(AttrValue::If)
        } else if lh.peek(syn::Ident) {
            let ident = input.parse::<Ident>()?;

            if ident == "Some" {
                let content;
                syn::parenthesized!(content in input);
                content.parse()
            } else if ident == "None" {
                Ok(AttrValue::None)
            } else {
                Err(syn::Error::new_spanned(ident, "expected `Some` or `None`"))
            }
        } else {
            Err(lh.error())
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Unit;

impl Parse for Unit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        if content.is_empty() {
            Ok(Self)
        } else {
            Err(content.error("expected empty tuple"))
        }
    }
}

#[derive(Debug, Clone)]
struct If<T> {
    cond: syn::Expr,
    then_tree: T,
    else_tree: Option<ElseBranch<T>>,
}

#[derive(Debug, Clone)]
enum ElseBranch<T> {
    If(Box<If<T>>),
    Else(T),
}

impl<T> Parse for If<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![if]>()?;

        let cond = input.parse::<syn::Expr>()?;

        let content;
        syn::braced!(content in input);
        let then_tree = content.parse::<T>()?;

        let else_tree = if input.parse::<Token![else]>().is_ok() {
            if let Ok(else_if) = input.parse::<Self>() {
                Some(ElseBranch::If(Box::new(else_if)))
            } else {
                let content;
                syn::braced!(content in input);
                Some(content.parse::<T>().map(ElseBranch::Else)?)
            }
        } else {
            None
        };

        Ok(Self {
            cond,
            then_tree,
            else_tree,
        })
    }
}

#[derive(Debug, Clone)]
struct For {
    pat: syn::Pat,
    expr: syn::Expr,
    tree: Tree,
}

impl Parse for For {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![for]>()?;
        let pat = input.parse::<syn::Pat>()?;

        input.parse::<Token![in]>()?;
        let expr = input.call(syn::Expr::parse_without_eager_brace)?;

        let content;
        syn::braced!(content in input);
        let tree = content.parse::<Tree>()?;

        Ok(Self { pat, expr, tree })
    }
}

#[derive(Debug, Clone)]
struct Match {
    expr: syn::Expr,
    arms: Vec<Arm>,
}

impl Parse for Match {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![match]>()?;

        let expr = input.call(syn::Expr::parse_without_eager_brace)?;

        let content;
        syn::braced!(content in input);
        let mut arms = Vec::new();
        while !content.is_empty() {
            arms.push(content.call(Arm::parse)?);
        }

        Ok(Self { expr, arms })
    }
}

#[derive(Debug, Clone)]
struct Arm {
    pat: syn::Pat,
    guard: Option<syn::Expr>,
    tree: Tree,
}

impl Parse for Arm {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pat = input.parse::<syn::Pat>()?;

        let guard = if input.parse::<Token![if]>().is_ok() {
            let expr = input.parse::<syn::Expr>()?;
            Some(expr)
        } else {
            None
        };

        input.parse::<Token![=>]>()?;

        let mut nodes = Vec::new();
        while input.fork().parse::<HtmlNode>().is_ok() {
            let node = input.parse::<HtmlNode>()?;
            nodes.push(node);
        }

        input.parse::<Token![,]>()?;

        Ok(Self {
            pat,
            guard,
            tree: Tree { nodes },
        })
    }
}

#[derive(Debug, Clone)]
struct Close(Ident);

impl Parse for Close {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

// like `std::write` but infallible
macro_rules! write {
    ( $($tt:tt)* ) => {
        std::write!($($tt)*).unwrap()
    };
}

impl ToTokens for Tree {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        let mut inside_braces = TokenStream::new();

        inside_braces.extend(quote! {
            let mut html = axum_liveview::html::__private::html();
        });

        nodes_to_tokens(self.nodes.clone(), &mut inside_braces);

        out.extend(quote! {
            {
                #inside_braces
                html
            }
        });
    }
}

fn nodes_to_tokens(nodes: Vec<HtmlNode>, out: &mut TokenStream) {
    let mut buf = String::new();
    for node in nodes {
        node.node_to_tokens(&mut buf, out)
    }
    out.extend(quote! {
        axum_liveview::html::__private::fixed(&mut html, #buf);
    });
}

trait NodeToTokens {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream);
}

impl NodeToTokens for Tree {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        for node in &self.nodes {
            node.node_to_tokens(buf, out)
        }
        out.extend(quote! {
            axum_liveview::html::__private::fixed(&mut html, #buf);
        });
    }
}

impl NodeToTokens for HtmlNode {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        match self {
            HtmlNode::Doctype(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::TagNode(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::LitStr(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::Block(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::If(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::For(inner) => inner.node_to_tokens(buf, out),
            HtmlNode::Match(inner) => inner.node_to_tokens(buf, out),
        }
    }
}

impl NodeToTokens for Doctype {
    fn node_to_tokens(&self, buf: &mut String, _out: &mut TokenStream) {
        write!(buf, "<!DOCTYPE html>");
    }
}

impl NodeToTokens for TagNode {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        let Self { open, attrs, close } = self;

        write!(buf, "<{}", open);

        if !attrs.is_empty() {
            attrs.node_to_tokens(buf, out);
        }

        write!(buf, ">");
        if let Some(TagClose {
            inner: inner_nodes,
            close,
        }) = close
        {
            for node in inner_nodes {
                node.node_to_tokens(buf, out);
            }

            close.node_to_tokens(buf, out);
        }
    }
}

impl NodeToTokens for Vec<Attr> {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        let mut first = true;
        let mut attrs = self.into_iter().peekable();
        while let Some(attr) = attrs.next() {
            if let AttrValue::None = &attr.value {
                continue;
            }

            if first {
                write!(buf, " ");
                first = false;
            }

            write!(buf, "{}", attr.ident.ident);

            attr.value.node_to_tokens(buf, out);

            if attrs.peek().is_some() {
                write!(buf, " ");
            }
        }
    }
}

impl NodeToTokens for AttrValue {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        match self {
            AttrValue::LitStr(lit_str) => {
                write!(buf, "=");
                write!(buf, "{:?}", lit_str.value());
            }
            AttrValue::Block(block) => {
                write!(buf, "=");
                out.extend(quote! {
                    axum_liveview::html::__private::fixed(&mut html, #buf);
                });
                buf.clear();
                out.extend(quote! {
                    // TODO(david): using `Debug` to escape qoutes
                    // not sure if thats ideal. Do we need to consider newlines
                    // etc?
                    #[allow(unused_braces)]
                    axum_liveview::html::__private::dynamic(
                        &mut html,
                        format!("{:?}", #block),
                    );
                });
            }
            AttrValue::If(If {
                cond,
                then_tree,
                else_tree,
            }) => {
                write!(buf, "=");
                out.extend(quote! {
                    axum_liveview::html::__private::fixed(&mut html, #buf);
                });
                buf.clear();

                println!("{}", out.to_string());
                todo!("if_tree")
            }
            AttrValue::Unit(_) => {}
            AttrValue::None => unreachable!(),
        }
    }
}

impl NodeToTokens for LitStr {
    fn node_to_tokens(&self, buf: &mut String, _out: &mut TokenStream) {
        write!(buf, "{}", self.value());
    }
}

impl NodeToTokens for Block {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        out.extend(quote! {
            axum_liveview::html::__private::fixed(&mut html, #buf);
        });
        buf.clear();

        out.extend(quote! {
            #[allow(unused_braces)]
            axum_liveview::html::__private::dynamic(&mut html, #self);
        });
    }
}

impl NodeToTokens for If<Tree> {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        let Self {
            cond,
            then_tree,
            else_tree,
        } = self;

        out.extend(quote! {
            axum_liveview::html::__private::fixed(&mut html, #buf);
        });
        buf.clear();

        if let Some(else_tree) = else_tree {
            let else_tree = match else_tree {
                ElseBranch::If(else_if) => Tree {
                    nodes: vec![HtmlNode::If(*else_if.clone())],
                },
                ElseBranch::Else(else_) => else_.clone(),
            };
            out.extend(quote! {
                if #cond {
                    axum_liveview::html::__private::dynamic(&mut html, #then_tree);
                } else {
                    axum_liveview::html::__private::dynamic(&mut html, #else_tree);
                }
            });
        } else {
            out.extend(quote! {
                if #cond {
                    axum_liveview::html::__private::dynamic(&mut html, #then_tree);
                } else {
                    axum_liveview::html::__private::dynamic(&mut html, "");
                }
            });
        }
    }
}

impl NodeToTokens for For {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        let Self { pat, expr, tree } = self;

        out.extend(quote! {
            axum_liveview::html::__private::fixed(&mut html, #buf);
        });
        buf.clear();

        out.extend(quote! {
            let mut __first = true;
            for #pat in #expr {
                axum_liveview::html::__private::dynamic(&mut html, #tree);

                // add some empty segments so the number of placeholders matches up
                if !__first {
                    axum_liveview::html::__private::fixed(&mut html, "");
                }
                __first = false;
            }
        })
    }
}

impl NodeToTokens for Match {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        let Match { expr, arms } = self;

        out.extend(quote! {
            axum_liveview::html::__private::fixed(&mut html, #buf);
        });
        buf.clear();

        let arms = arms
            .iter()
            .map(|Arm { pat, guard, tree }| {
                let guard = guard.as_ref().map(|guard| quote! { if #guard });
                quote! {
                    #pat #guard => axum_liveview::html::__private::dynamic(&mut html, #tree),
                }
            })
            .collect::<TokenStream>();

        out.extend(quote! {
            match #expr {
                #arms
            }
        })
    }
}

impl NodeToTokens for Close {
    fn node_to_tokens(&self, buf: &mut String, out: &mut TokenStream) {
        write!(buf, "</{}>", self.0);
    }
}
