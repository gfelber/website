use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::sync::Mutex;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input,
  punctuated::Punctuated,
  Expr, ExprLit, ItemFn, Lit, Token,
  parse_quote
};

// A static mutable global map to keep track of registered commands
static REGISTERED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

struct ShellCmdArgs {
  commands: Expr,
  help_msg: String,
  mobile: Expr,
}

impl Parse for ShellCmdArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let args: Punctuated<Expr, Token![,]> = input.parse_terminated(Expr::parse, Token![,])?;

    if args.len() < 2 {
      return Err(syn::Error::new(
        input.span(),
        "expected at least 2 arguments: COMMANDS, help_message",
      ));
    }

    let commands = args[0].clone();

    let help_msg = if let Expr::Lit(ExprLit {
      lit: Lit::Str(lit_str),
      ..
    }) = &args[1]
    {
      lit_str.value()
    } else {
      return Err(syn::Error::new_spanned(
        &args[1],
        "expected a string literal",
      ));
    };

    let mobile: Expr = if args.len() > 2 {
      args[2].clone()
    } else {
      parse_quote!(MobileType::NotMobile)
    };

    Ok(ShellCmdArgs {
      commands,
      help_msg,
      mobile,
    })
  }
}

#[proc_macro_attribute]
pub fn shell_cmd(attr: TokenStream, item: TokenStream) -> TokenStream {
  let args = parse_macro_input!(attr as ShellCmdArgs);
  // Parse the input as `ItemFn`
  let item = parse_macro_input!(item as ItemFn);

  let ItemFn {
    sig,
    vis,
    block,
    attrs,
  } = item;

  let statements = block.stmts;
  let function_identifier = &sig.ident;
  let function_name = function_identifier.to_string();

  REGISTERED_COMMANDS
    .lock()
    .unwrap()
    .push(function_name.clone());

  let register_function_name = format_ident!("__register_{}", function_name);

  let commands = args.commands;
  let help_msg = args.help_msg;
  let mobile = args.mobile;

  // Reconstruct the function as output using parsed input
  quote!(
    #(#attrs)*
    #vis #sig {
      let __result = {
        #(#statements)*
      };
      return __result;
    }

    pub fn #register_function_name() {
      info!("registering {}", #function_name);
      #commands.lock().unwrap().insert(#function_name, CmdInfo {
        func: #function_identifier,
        help: #help_msg,
        mobile: #mobile,
      });
    }

  )
  .into()
}

#[proc_macro_attribute]
pub fn cmds_init(_attr: TokenStream, item: TokenStream) -> TokenStream {
  // Parse the input as `ItemFn`
  let item = parse_macro_input!(item as ItemFn);

  let ItemFn {
    sig,
    vis,
    block,
    attrs,
  } = item;

  let statements = block.stmts;

  let register_statements: Vec<_> = REGISTERED_COMMANDS
    .lock()
    .unwrap()
    .iter()
    .map(|name| {
      let register_name = format_ident!("__register_{}", name);
      quote! { #register_name(); }
    })
    .collect();

  // Reconstruct the function as output using parsed input
  quote!(
    #(#attrs)*
    #vis #sig {
      #(#register_statements)*

      let __result = {
        #(#statements)*
      };

      return __result;
    }
  )
  .into()
}
