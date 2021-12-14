use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::{collections::VecDeque, fmt::Write};
use syn::{parse::Parse, punctuated::Punctuated, Block, Ident, LitStr, Token};

#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let tree = match syn::parse::<Tree>(input) {
        Ok(tree) => tree,
        Err(err) => return err.into_compile_error().into(),
    };

    tree.into_token_stream().into()
}

#[derive(Debug, Clone)]
struct Tree {
    nodes: Vec<HtmlNode>,
}

impl Parse for Tree {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let nodes = parse_many(input)?;
        Ok(Self { nodes })
    }
}

fn parse_many<P>(input: syn::parse::ParseStream) -> syn::Result<Vec<P>>
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
    If(If),
    For(For),
    Match(Match),
    Close(Ident),
}

impl Parse for HtmlNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
    close: Ident,
}

impl Parse for TagNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![<]>()?;
        let open = input.parse()?;

        let mut attrs = Vec::new();
        while input.peek(Ident) {
            let ident = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(input)?;
            let value = if input.parse::<Token![=]>().is_ok() {
                if let Ok(block) = input.parse().map(AttrValue::Block) {
                    Some(block)
                } else {
                    Some(input.parse().map(AttrValue::LitStr)?)
                }
            } else {
                None
            };
            attrs.push(Attr { ident, value });
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
        let close = input.parse::<Ident>()?;
        input.parse::<Token![>]>()?;

        if open != close {
            let span = open
                .span()
                .join(close.span())
                .unwrap_or_else(|| close.span());
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
    ident: Punctuated<Ident, Token![-]>,
    value: Option<AttrValue>,
}

impl Attr {
    fn ident(&self) -> String {
        let mut out = String::new();
        let mut iter = self.ident.iter().peekable();
        while let Some(ident) = iter.next() {
            if iter.peek().is_some() {
                let _ = write!(out, "{}-", ident);
            } else {
                let _ = write!(out, "{}", ident);
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
enum AttrValue {
    LitStr(LitStr),
    Block(Block),
}

#[derive(Debug, Clone)]
struct If {
    cond: syn::Expr,
    then_tree: Tree,
    else_tree: Option<Tree>,
}

impl Parse for If {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![if]>()?;

        let cond = input.parse::<syn::Expr>()?;

        let content;
        syn::braced!(content in input);
        let then_tree = content.parse::<Tree>()?;

        let else_tree = if input.parse::<Token![else]>().is_ok() {
            if input.peek(Token![if]) {
                // TODO(david): support `else if`
                return Err(input.error("`else if` is not supported yet"));
            }

            let content;
            syn::braced!(content in input);
            Some(content.parse::<Tree>()?)
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
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
    tree: Tree,
}

impl Parse for Arm {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let pat = input.parse::<syn::Pat>()?;

        if input.peek(Token![if]) {
            // TODO(david): support `if` guards
            return Err(input.error("`if` guards is not supported yet"));
        }

        input.parse::<Token![=>]>()?;

        let mut nodes = Vec::new();
        while input.fork().parse::<HtmlNode>().is_ok() {
            let node = input.parse::<HtmlNode>()?;
            nodes.push(node);
        }

        input.parse::<Token![,]>()?;

        Ok(Self {
            pat,
            tree: Tree { nodes },
        })
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
            let mut html = axum_liveview::html::Html::default();
        });

        nodes_to_tokens(self.nodes.iter().cloned().collect(), &mut inside_braces);

        out.extend(quote! {
            {
                #inside_braces
                html
            }
        });
    }
}

fn nodes_to_tokens(mut nodes_queue: VecDeque<HtmlNode>, out: &mut proc_macro2::TokenStream) {
    let mut buf = String::new();
    while let Some(node) = nodes_queue.pop_front() {
        match node {
            HtmlNode::TagNode(TagNode { open, attrs, close }) => {
                write!(buf, "<{}", open);

                if !attrs.is_empty() {
                    write!(buf, " ");

                    let mut attrs = attrs.iter().peekable();
                    while let Some(attr) = attrs.next() {
                        let ident = attr.ident();
                        write!(buf, "{}", ident);

                        match &attr.value {
                            Some(AttrValue::LitStr(lit_str)) => {
                                write!(buf, "=");
                                write!(buf, "{:?}", lit_str.value());
                            }
                            Some(AttrValue::Block(block)) => {
                                write!(buf, "=");
                                out.extend(quote! {
                                    axum_liveview::html::__private::fixed(&mut html, #buf);
                                });
                                buf.clear();
                                out.extend(quote! {
                                    #[allow(unused_braces)]
                                    axum_liveview::html::__private::dynamic(&mut html, #block);
                                });
                            }
                            None => {}
                        }

                        if attrs.peek().is_some() {
                            write!(buf, " ");
                        }
                    }
                }

                write!(buf, ">");
                if let Some(TagClose {
                    inner: inner_nodes,
                    close,
                }) = close
                {
                    nodes_queue.push_front(HtmlNode::Close(close.clone()));

                    for node in inner_nodes.iter().rev() {
                        nodes_queue.push_front(node.clone());
                    }
                }
            }
            HtmlNode::LitStr(lit_str) => {
                write!(buf, "{}", lit_str.value());
            }
            HtmlNode::Close(close) => {
                write!(buf, "</{}>", close);
            }
            HtmlNode::Block(block) => {
                out.extend(quote! {
                    axum_liveview::html::__private::fixed(&mut html, #buf);
                });
                buf.clear();

                out.extend(quote! {
                    #[allow(unused_braces)]
                    axum_liveview::html::__private::dynamic(&mut html, #block);
                });
            }
            HtmlNode::If(If {
                cond,
                then_tree,
                else_tree,
            }) => {
                out.extend(quote! {
                    axum_liveview::html::__private::fixed(&mut html, #buf);
                });
                buf.clear();

                if let Some(else_tree) = else_tree {
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
                        }
                    });
                }
            }
            HtmlNode::For(For { pat, expr, tree }) => {
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
            HtmlNode::Match(Match { expr, arms }) => {
                out.extend(quote! {
                    axum_liveview::html::__private::fixed(&mut html, #buf);
                });
                buf.clear();

                let arms = arms
                    .iter()
                    .map(|Arm { pat, tree }| {
                        quote! {
                            #pat => axum_liveview::html::__private::dynamic(&mut html, #tree),
                        }
                    })
                    .collect::<TokenStream>();

                out.extend(quote! {
                    match #expr {
                        #arms
                    }
                })
            }
            HtmlNode::Doctype(_) => {
                write!(buf, "<!DOCTYPE html>");
            }
        }
    }

    out.extend(quote! {
        axum_liveview::html::__private::fixed(&mut html, #buf);
    });
}
