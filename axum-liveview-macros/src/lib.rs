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

#[derive(Debug)]
struct Tree {
    nodes: Vec<HtmlNode>,
}

impl Parse for Tree {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut nodes = Vec::new();
        while !input.is_empty() {
            nodes.push(input.parse()?);
        }
        Ok(Self { nodes })
    }
}

#[derive(Debug, Clone)]
struct HtmlNode {
    open: Ident,
    attrs: Vec<Attr>,
    close: Option<NodeClose>,
}

#[derive(Debug, Clone)]
struct NodeClose {
    inner: Option<NodeInner>,
    close: Ident,
}

impl Parse for HtmlNode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        input.parse::<Token![<]>()?;
        let open = input.parse::<Ident>()?;

        let mut attrs = Vec::new();
        while input.peek(Ident) {
            let ident = Punctuated::<Ident, Token![-]>::parse_separated_nonempty(input)?;
            let value = if input.parse::<Token![=]>().is_ok() {
                if let Ok(block) = input.parse::<Block>().map(AttrValue::Block) {
                    Some(block)
                } else {
                    Some(input.parse::<LitStr>().map(AttrValue::LitStr)?)
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

        let inner = if let Ok(lit_str) = input.parse::<LitStr>().map(NodeInner::LitStr) {
            Some(lit_str)
        } else if input.peek(Token![<]) && !input.peek2(Token![/]) {
            let mut nodes = Vec::new();
            while input.peek(Token![<]) && !input.peek2(Token![/]) {
                nodes.push(input.parse::<Self>()?);
            }
            Some(NodeInner::Nodes(nodes))
        } else if let Ok(block) = input.parse::<Block>().map(NodeInner::Block) {
            Some(block)
        } else {
            None
        };

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let close = input.parse::<Ident>()?;
        input.parse::<Token![>]>()?;

        if open != close {
            let span = open.span().join(close.span()).unwrap_or_else(|| close.span());
            return Err(syn::Error::new(span, "Unmatched close tag"));
        }

        Ok(Self {
            open,
            attrs,
            close: Some(NodeClose { inner, close }),
        })
    }
}

#[derive(Debug, Clone)]
enum NodeInner {
    LitStr(LitStr),
    Block(Block),
    Nodes(Vec<HtmlNode>),
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

// like `std::write` but infallible
macro_rules! write {
    ( $($tt:tt)* ) => {
        std::write!($($tt)*).unwrap()
    };
}

impl ToTokens for Tree {
    fn to_tokens(&self, out: &mut proc_macro2::TokenStream) {
        nodes_to_tokens(self.nodes.iter().collect(), out)
    }
}

fn nodes_to_tokens(mut nodes_queue: VecDeque<&HtmlNode>, out: &mut proc_macro2::TokenStream) {
    let mut tokens = TokenStream::new();
    tokens.extend(quote! {
        let mut view = axum_liveview::View::default();
    });

    let mut buf = String::new();
    while let Some(node) = nodes_queue.pop_front() {
        let HtmlNode { open, attrs, close } = node;

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
                        tokens.extend(quote! { view.fixed(#buf); });
                        buf.clear();
                        tokens.extend(quote! { view.dynamic(#block); });
                    }
                    None => {}
                }

                if attrs.peek().is_some() {
                    write!(buf, " ");
                }
            }
        }

        if let Some(NodeClose { inner, close }) = close {
            write!(buf, ">");

            if let Some(inner) = inner {
                match inner {
                    NodeInner::LitStr(lit_str) => {
                        write!(buf, "{}", lit_str.value());
                    }
                    NodeInner::Block(block) => {
                        tokens.extend(quote! { view.fixed(#buf); });
                        buf.clear();
                        tokens.extend(quote! { view.dynamic(#block); });
                    }
                    NodeInner::Nodes(inner_nodes) => {
                        nodes_queue.reserve(inner_nodes.len());
                        for node in inner_nodes.iter().rev() {
                            nodes_queue.push_front(node);
                        }
                        continue;
                    }
                }
            }

            write!(buf, "</{}>", close);
        } else {
            write!(buf, " />");
        }
    }

    if !buf.is_empty() {
        tokens.extend(quote! { view.fixed(#buf); });
    }

    out.extend(quote! {
        #[allow(unused_braces)]
        {
            #tokens
            view
        }
    });
}
