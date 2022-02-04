//! Internal macro crate for [axum-live-view].
//!
//! [axum-live-view]: https://crates.io/crates/axum-live-view

#![warn(
    clippy::all,
    clippy::dbg_macro,
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
    missing_debug_implementations,
    missing_docs
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
    spanned::Spanned,
    Block, Ident, LitStr, Token,
};

#[proc_macro]
#[allow(missing_docs)]
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
        loop {
            if input.peek(Token![/]) || input.peek(Token![>]) || input.is_empty() {
                break;
            }

            match input.fork().parse::<AttrIdent>() {
                Ok(_) => attrs.push(input.parse()?),
                Err(err) => {
                    return Err(err);
                }
            }
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
enum Attr {
    Normal {
        ident: AttrIdent,
        value: NormalAttrValue,
    },
    Axm {
        ident: AttrIdent,
        value: AxmAttrValue,
    },
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<AttrIdent>()?;

        match ident {
            AttrIdent::Lit(_) => {
                let value = if input.parse::<Token![=]>().is_ok() {
                    input.parse()?
                } else {
                    NormalAttrValue::Unit(Unit)
                };
                Ok(Self::Normal { ident, value })
            }
            AttrIdent::Axm(_) => {
                input.parse::<Token![=]>()?;
                Ok(Self::Axm {
                    ident,
                    value: input.parse()?,
                })
            }
        }
    }
}

#[derive(Debug, Clone)]
enum AttrIdent {
    Lit(String),
    Axm(String),
}

impl Parse for AttrIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.parse::<Token![type]>().is_ok() {
            Ok(Self::Lit("type".to_owned()))
        } else if input.parse::<Token![for]>().is_ok() {
            Ok(Self::Lit("for".to_owned()))
        } else {
            let idents = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(input)?;
            let idents_span = idents.span();
            let mut out = String::new();
            let mut iter = idents.iter().peekable();
            while let Some(ident) = iter.next() {
                if iter.peek().is_some() {
                    let _ = write!(out, "{}-", ident);
                } else {
                    let _ = write!(out, "{}", ident);
                }
            }

            match out.strip_prefix("axm-") {
                Some(ident) => match ident {
                    "click" | "input" | "change" | "submit" | "focus" | "blur" | "keydown"
                    | "keyup" | "window-keydown" | "window-keyup" | "window-focus"
                    | "window-blur" | "mouseenter" | "mouseover" | "mouseleave" | "mouseout"
                    | "mousemove" | "scroll" => Ok(Self::Axm(out)),
                    "throttle" | "debounce" | "key" => Ok(Self::Lit(out)),
                    _ => Err(syn::Error::new(
                        idents_span,
                        format!("unknown `{out}` attribute"),
                    )),
                },
                None => Ok(Self::Lit(out)),
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum NormalAttrValue {
    LitStr(LitStr),
    Block(Block),
    Unit(Unit),
    If(If<Box<NormalAttrValue>>),
    None,
}

impl Parse for NormalAttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lh = input.lookahead1();

        if lh.peek(syn::token::Brace) {
            input.parse().map(NormalAttrValue::Block)
        } else if lh.peek(syn::token::Paren) {
            input.parse().map(NormalAttrValue::Unit)
        } else if lh.peek(syn::LitStr) {
            input.parse().map(NormalAttrValue::LitStr)
        } else if lh.peek(Token![if]) {
            input.parse().map(NormalAttrValue::If)
        } else if lh.peek(syn::Ident) {
            let ident = input.parse::<Ident>()?;

            if ident == "Some" {
                let content;
                syn::parenthesized!(content in input);
                content.parse()
            } else if ident == "None" {
                Ok(NormalAttrValue::None)
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
#[allow(clippy::large_enum_variant)]
enum AxmAttrValue {
    Block(Block),
    If(If<Box<AxmAttrValue>>),
}

impl Parse for AxmAttrValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lh = input.lookahead1();

        if lh.peek(syn::token::Brace) {
            input.parse().map(AxmAttrValue::Block)
        } else if lh.peek(Token![if]) {
            let inner = input
                .parse::<If<syn::Expr>>()?
                .map(|expr: syn::Expr| Box::new(AxmAttrValue::Block(syn::parse_quote!({ #expr }))));
            Ok(AxmAttrValue::If(inner))
        } else {
            Err(lh.error())
        }
    }
}

#[derive(Debug, Clone)]
struct If<T> {
    cond: syn::Expr,
    then_tree: T,
    else_tree: Option<ElseBranch<T>>,
}

impl<T> If<T> {
    fn map<F, K>(self, f: F) -> If<K>
    where
        F: Fn(T) -> K,
    {
        let then_tree = f(self.then_tree);
        let else_tree = self.else_tree.map(|else_tree| match else_tree {
            ElseBranch::If(if_) => ElseBranch::If(Box::new(if_.map(f))),
            ElseBranch::Else(else_) => ElseBranch::Else(f(else_)),
        });

        If {
            cond: self.cond,
            then_tree,
            else_tree,
        }
    }
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

        let cond = input.call(syn::Expr::parse_without_eager_brace)?;

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

trait NodeToTokens {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream);
}

#[derive(Default, Debug)]
struct FixedParts {
    parts: Vec<String>,
}

impl FixedParts {
    fn append(&mut self, part: impl AsRef<str>) {
        loop {
            if let Some(last) = self.parts.last_mut() {
                last.push_str(part.as_ref());
                return;
            } else {
                self.parts.push(String::new());
            }
        }
    }

    fn start_new_part(&mut self) {
        if self.parts.is_empty() {
            self.append("");
        }
        self.parts.push(String::new());
    }
}

impl<T> NodeToTokens for &T
where
    T: NodeToTokens,
{
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        NodeToTokens::node_to_tokens(*self, fixed, out)
    }
}

impl<T> NodeToTokens for Box<T>
where
    T: NodeToTokens,
{
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        NodeToTokens::node_to_tokens(&**self, fixed, out)
    }
}

impl<T> NodeToTokens for Vec<T>
where
    T: NodeToTokens,
{
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        for node in self {
            node.node_to_tokens(fixed, out)
        }
    }
}

struct ToTokensViaNodeToTokens<T>(T);

impl<T> ToTokens for ToTokensViaNodeToTokens<T>
where
    T: NodeToTokens,
{
    fn to_tokens(&self, out: &mut TokenStream) {
        let mut inside_braces = TokenStream::new();

        inside_braces.extend(quote! {
            let mut __dynamic = std::vec::Vec::<axum_live_view::__private::DynamicFragment<_>>::new();
        });

        let mut fixed = FixedParts::default();
        self.0.node_to_tokens(&mut fixed, &mut inside_braces);
        let FixedParts { parts } = fixed;

        out.extend(quote! {
            {
                use axum_live_view::__private::DynamicFragmentVecExt;
                #inside_braces
                axum_live_view::__private::HtmlBuilder {
                    dynamic: __dynamic,
                    fixed: &[#(#parts),*],
                }.into_html()
            }
        });
    }
}

impl ToTokens for Tree {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        ToTokensViaNodeToTokens(self).to_tokens(out)
    }
}

impl NodeToTokens for Tree {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        for node in &self.nodes {
            node.node_to_tokens(fixed, out)
        }
    }
}

impl NodeToTokens for HtmlNode {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        match self {
            HtmlNode::Doctype(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::TagNode(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::LitStr(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::Block(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::If(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::For(inner) => inner.node_to_tokens(fixed, out),
            HtmlNode::Match(inner) => inner.node_to_tokens(fixed, out),
        }
    }
}

impl NodeToTokens for Doctype {
    fn node_to_tokens(&self, fixed: &mut FixedParts, _out: &mut TokenStream) {
        fixed.append("<!DOCTYPE html>".to_owned());
    }
}

impl NodeToTokens for TagNode {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        let Self { open, attrs, close } = self;

        fixed.append(format!("<{}", open));

        if !attrs.is_empty() {
            attrs.node_to_tokens(fixed, out);
        }

        fixed.append(">".to_owned());
        if let Some(TagClose {
            inner: inner_nodes,
            close,
        }) = close
        {
            for node in inner_nodes {
                node.node_to_tokens(fixed, out);
            }

            close.node_to_tokens(fixed, out);
        }
    }
}

impl NodeToTokens for Attr {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        match self {
            Attr::Normal { ident, value } => match value {
                NormalAttrValue::LitStr(lit_str) => {
                    fixed.append(" ".to_owned());
                    ident.node_to_tokens(fixed, out);
                    fixed.append(format!("={:?}", lit_str.value()));
                }
                NormalAttrValue::Block(block) => {
                    fixed.append(" ".to_owned());
                    ident.node_to_tokens(fixed, out);
                    fixed.append("=".to_owned());
                    fixed.start_new_part();
                    out.extend(quote! {
                        #[allow(unused_braces)]
                        __dynamic.push_fragment(format!("{:?}", #block));
                    });
                }
                NormalAttrValue::If(if_) => {
                    let if_ = if_.clone().map(|attr_value| Self::Normal {
                        ident: ident.clone(),
                        value: *attr_value,
                    });
                    if_.node_to_tokens(fixed, out);
                }
                NormalAttrValue::Unit(_) => {
                    fixed.append(" ".to_owned());
                    ident.node_to_tokens(fixed, out);
                }
                NormalAttrValue::None => {}
            },
            Attr::Axm { ident, value } => match value {
                AxmAttrValue::Block(block) => {
                    fixed.append(" ".to_owned());
                    ident.node_to_tokens(fixed, out);
                    fixed.append("=".to_owned());
                    fixed.start_new_part();
                    out.extend(quote! {
                        #[allow(unused_braces)]
                        __dynamic.push_message(#block);
                    });
                }
                AxmAttrValue::If(if_) => {
                    let if_ = if_.clone().map(|attr_value| Self::Axm {
                        ident: ident.clone(),
                        value: *attr_value,
                    });
                    if_.node_to_tokens(fixed, out);
                }
            },
        }
    }
}

impl NodeToTokens for AttrIdent {
    fn node_to_tokens(&self, fixed: &mut FixedParts, _out: &mut TokenStream) {
        match self {
            AttrIdent::Lit(ident) => fixed.append(ident.to_string()),
            AttrIdent::Axm(ident) => fixed.append(ident.to_string()),
        }
    }
}

impl NodeToTokens for LitStr {
    fn node_to_tokens(&self, fixed: &mut FixedParts, _out: &mut TokenStream) {
        fixed.append(self.value());
    }
}

impl NodeToTokens for Block {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        fixed.start_new_part();

        out.extend(quote! {
            #[allow(unused_braces)]
            __dynamic.push_fragment(#self);
        });
    }
}

impl<T> NodeToTokens for If<T>
where
    T: NodeToTokens,
{
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        let Self {
            cond,
            then_tree,
            else_tree,
        } = self;

        fixed.start_new_part();

        let then_tree = ToTokensViaNodeToTokens(then_tree);

        if let Some(else_tree) = else_tree {
            match else_tree {
                ElseBranch::If(else_if) => {
                    let else_if = ToTokensViaNodeToTokens(else_if);
                    out.extend(quote! {
                        if #cond {
                            __dynamic.push_fragment(#then_tree);
                        } else {
                            __dynamic.push_fragment(#else_if);
                        }
                    });
                }
                ElseBranch::Else(else_) => {
                    let else_ = ToTokensViaNodeToTokens(else_);
                    out.extend(quote! {
                        if #cond {
                            __dynamic.push_fragment(#then_tree);
                        } else {
                            __dynamic.push_fragment(#else_);
                        }
                    });
                }
            };
        } else {
            out.extend(quote! {
                if #cond {
                    __dynamic.push_fragment(#then_tree);
                } else {
                    __dynamic.push_fragment("");
                }
            });
        }
    }
}

impl NodeToTokens for For {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        let Self { pat, expr, tree } = self;

        fixed.start_new_part();

        let mut inside = TokenStream::new();

        let mut fixed = FixedParts::default();
        tree.node_to_tokens(&mut fixed, &mut inside);
        let FixedParts { parts } = fixed;

        out.extend(quote! {
            let mut __dynamic_loop_parts = Vec::new();
            for #pat in #expr {
                let __parts = {
                    let mut __dynamic = std::vec::Vec::<axum_live_view::__private::DynamicFragment<_>>::new();
                    #inside
                    __dynamic
                };
                __dynamic_loop_parts.push(__parts);
            }
            __dynamic.push_fragments(
                &[#(#parts),*],
                __dynamic_loop_parts,
            );
        });
    }
}

impl NodeToTokens for Match {
    fn node_to_tokens(&self, fixed: &mut FixedParts, out: &mut TokenStream) {
        let Match { expr, arms } = self;

        fixed.start_new_part();

        let arms = arms
            .iter()
            .map(|Arm { pat, guard, tree }| {
                let guard = guard.as_ref().map(|guard| quote! { if #guard });
                quote! {
                    #pat #guard => __dynamic.push_fragment(#tree),
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
    fn node_to_tokens(&self, fixed: &mut FixedParts, _out: &mut TokenStream) {
        fixed.append(format!("</{}>", self.0));
    }
}
