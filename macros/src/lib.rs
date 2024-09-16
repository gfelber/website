use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::sync::Mutex;
use syn::{
  parse::{Parse, ParseStream},
  parse_macro_input,
  punctuated::Punctuated,
  Expr, ExprLit, ItemFn, Lit, Token,
};

// A static mutable global map to keep track of registered commands
static REGISTERED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

struct ShellCmdArgs {
  commands: Expr,
  helps: Expr,
  help_msg: String,
}

impl Parse for ShellCmdArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let args: Punctuated<Expr, Token![,]> = input.parse_terminated(Expr::parse, Token![,])?;

    let commands = args[0].clone();

    let helps = args[1].clone();

    let other = if let Expr::Lit(ExprLit {
      lit: Lit::Str(lit_str),
      ..
    }) = &args[2]
    {
      lit_str.value()
    } else {
      return Err(syn::Error::new_spanned(
        &args[2],
        "expected a string literal",
      ));
    };

    Ok(ShellCmdArgs {
      commands,
      helps,
      help_msg: other,
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
  let helps = args.helps;
  let help_msg = args.help_msg;

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
      #commands.lock().unwrap().insert(#function_name, #function_identifier);
      #helps.lock().unwrap().push(#help_msg);
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
