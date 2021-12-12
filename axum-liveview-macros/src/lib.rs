use quote::{ToTokens, quote};

#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = input.into_iter().map(|tree| {
        tree.to_string()
    }).collect::<String>();

    html_from_str(&input)
}

fn html_from_str(input: &str) -> proc_macro::TokenStream {
    let mut tokens = Vec::new();
    for c in input.chars() {
        match c {
            '{' => tokens.push(Token::BraceOpen(BraceOpen)),
            '}' => tokens.push(Token::BraceClose(BraceClose)),
            _ => tokens.push(Token::Char(c)),
        }
    }

    let mut stream = TokenStream { tokens, pos: 0 };

    let tree = stream.parse::<Tree>().unwrap();

    if !stream.is_empty() {
        panic!("Unexpected token");
    }

    let tokens = quote! { #tree };
    println!("{}", tokens);
    (tokens).into()
}

struct TokenStream {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenStream {
    fn is_empty(&self) -> bool {
        self.tokens.get(self.pos).is_none()
    }

    fn parse<P>(&mut self) -> Result<P, ParseError>
    where
        P: Parse,
    {
        P::parse(self)
    }

    fn parse_many<P>(&mut self) -> Vec<P>
    where
        P: Parse,
    {
        let mut out = Vec::new();
        while let Some(token) = self.try_parse() {
            out.push(token);
        }
        out
    }

    #[allow(warnings)]
    fn parse_until<P, I>(&mut self) -> Vec<P>
    where
        P: Parse,
        I: Parse,
    {
        let mut out = Vec::new();
        loop {
            if self.peek::<I>() {
                break;
            } else {
                let nodes = self.parse_many();
                out.extend(nodes);
            }
        }
        out
    }

    fn try_parse<P>(&mut self) -> Option<P>
    where
        P: Parse,
    {
        let pos = self.pos;
        if let Ok(node) = P::parse(self) {
            Some(node)
        } else {
            self.pos = pos;
            None
        }
    }

    fn peek<P>(&mut self) -> bool
    where
        P: Parse,
    {
        let pos = self.pos;
        let out = P::parse(self).is_ok();
        self.pos = pos;
        out
    }
}

#[derive(Debug, Clone, Copy)]
enum Token {
    BraceOpen(BraceOpen),
    BraceClose(BraceClose),
    Char(char),
}

#[derive(Debug, Clone, Copy)]
struct BraceOpen;

#[derive(Debug, Clone, Copy)]
struct BraceClose;

impl Iterator for TokenStream {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.tokens.get(self.pos).copied();
        self.pos += 1;
        token
    }
}

trait Parse: Sized {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError>;
}

impl Parse for BraceOpen {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        if let Some(Token::BraceOpen(token)) = stream.next() {
            Ok(token)
        } else {
            Err(ParseError)
        }
    }
}

impl Parse for BraceClose {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        if let Some(Token::BraceClose(token)) = stream.next() {
            Ok(token)
        } else {
            Err(ParseError)
        }
    }
}

impl Parse for char {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        if let Some(Token::Char(token)) = stream.next() {
            Ok(token)
        } else {
            Err(ParseError)
        }
    }
}

#[derive(Debug)]
struct ParseError;

#[derive(Debug)]
enum Node {
    Fixed(Fixed),
    Dynamic(Box<Dynamic>),
}

impl Parse for Node {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        if let Some(node) = stream.try_parse() {
            return Ok(Self::Fixed(node));
        }

        if let Some(node) = stream.try_parse() {
            return Ok(Self::Dynamic(Box::new(node)));
        }

        Err(ParseError)
    }
}

impl ToTokens for Node {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Node::Fixed(inner) => inner.to_tokens(tokens),
            Node::Dynamic(inner) => inner.to_tokens(tokens),
        }
    }
}

#[derive(Debug)]
struct Fixed(String);

impl Parse for Fixed {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        if stream.peek::<char>() {
            let chars = stream.parse_many::<char>().into_iter().collect();
            Ok(Self(chars))
        } else {
            Err(ParseError)
        }
    }
}

impl ToTokens for Fixed {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let value = &self.0;
        tokens.extend(quote! {
            view.fixed(#value);
        })
    }
}

#[derive(Debug)]
struct Dynamic(syn::Expr);

impl Parse for Dynamic {
    fn parse(mut stream: &mut TokenStream) -> Result<Self, ParseError> {
        if stream.peek::<BraceOpen>() {
            stream.parse::<BraceOpen>()?;

            let mut depth: u32 = 0;

            let mut expr = String::new();

            for token in &mut stream {
                match token {
                    Token::BraceOpen(_) => {
                        depth += 1;
                        expr.push('{');
                    }
                    Token::BraceClose(_) => {
                        if depth == 0 {
                            break;
                        } else {
                            depth -= 1;
                            expr.push('}');
                        }
                    }
                    Token::Char(c) => expr.push(c),
                }
            }

            let expr = syn::parse_str::<syn::Expr>(&expr).unwrap();

            Ok(Self(expr))
        } else {
            Err(ParseError)
        }
    }
}

impl ToTokens for Dynamic {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let expr = &self.0;
        tokens.extend(quote! {
            view.dynamic({
                #expr
            });
        })
    }
}

#[derive(Debug)]
struct Tree {
    nodes: Vec<Node>,
}

impl Parse for Tree {
    fn parse(stream: &mut TokenStream) -> Result<Self, ParseError> {
        let nodes = stream.parse_many();
        Ok(Self { nodes })
    }
}

impl ToTokens for Tree {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let nodes = &self.nodes;

        tokens.extend(quote! {
            {
                let mut view = axum_liveview::View::default();
                #(#nodes)*
                view
            }
        })
    }
}
