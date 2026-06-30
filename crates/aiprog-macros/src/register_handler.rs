use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Expr, Ident, LitStr, Token};

pub fn register_handler_impl(input: TokenStream) -> TokenStream {
    let RegisterHandlerInput {
        registry,
        path,
        handler,
    } = parse_macro_input!(input as RegisterHandlerInput);

    let handler_struct = format_ident!("__AiprogHandler_{}", handler);

    let expanded = quote! {
        #registry.register_handler(#path, #handler_struct)
    };

    TokenStream::from(expanded)
}

struct RegisterHandlerInput {
    registry: Expr,
    path: LitStr,
    handler: Ident,
}

impl syn::parse::Parse for RegisterHandlerInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let registry: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let handler: Ident = input.parse()?;

        if !input.is_empty() {
            return Err(input.error("unexpected extra tokens after handler ident"));
        }

        Ok(RegisterHandlerInput { registry, path, handler })
    }
}
