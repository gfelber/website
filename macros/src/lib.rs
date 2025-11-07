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

static REGISTERED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

struct ShellCmdArgs {
  commands: Expr,
  help_msg: String,
  name: Option<String>,
  cmd_type: Expr,
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

    let help_msg = if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = &args[1] {
      lit_str.value()
    } else {
      return Err(syn::Error::new_spanned(&args[1], "help message must be a string literal"));
    };

    let mut cmd_type: Expr = parse_quote!(CmdType::NotMobile);
    let mut name: Option<String> = None;

    // Parse named arguments
    for arg in args.iter().skip(2) {
      let Expr::Assign(assign) = arg else {
        return Err(syn::Error::new_spanned(arg, "use named parameters: cmd_type=... or name=..."));
      };

      let Expr::Path(path) = &*assign.left else {
        return Err(syn::Error::new_spanned(&assign.left, "expected parameter name"));
      };

      let param_name = path.path.segments.last().unwrap().ident.to_string();
      
      match param_name.as_str() {
        "cmd_type" => {
          cmd_type = (*assign.right).clone();
        }
        "name" => {
          if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = &*assign.right {
            name = Some(lit_str.value());
          } else {
            return Err(syn::Error::new_spanned(&assign.right, "name must be a string literal"));
          }
        }
        _ => {
          return Err(syn::Error::new_spanned(path, format!("unknown parameter: {}", param_name)));
        }
      }
    }

    Ok(ShellCmdArgs { commands, help_msg, name, cmd_type })
  }
}

#[proc_macro_attribute]
pub fn shell_cmd(attr: TokenStream, item: TokenStream) -> TokenStream {
  let args = parse_macro_input!(attr as ShellCmdArgs);
  let input_fn = parse_macro_input!(item as ItemFn);

  let ItemFn { sig, vis, block, attrs } = input_fn;
  
  let function_name = sig.ident.to_string();
  let function_ident = &sig.ident;
  
  REGISTERED_COMMANDS.lock().unwrap().push(function_name.clone());

  let register_fn_name = format_ident!("__register_{}", function_name);
  let commands = &args.commands;
  let help_msg = &args.help_msg;
  let cmd_type = &args.cmd_type;
  let cmd_name = args.name.as_ref().unwrap_or(&function_name);

  quote!(
    #(#attrs)*
    #vis #sig #block

    pub fn #register_fn_name() {
      info!("registering {}", #cmd_name);
      #commands.lock().unwrap().insert(
        #cmd_name,
        CmdInfo {
          func: #function_ident,
          help: #help_msg,
          cmd_type: #cmd_type,
        }
      );
    }
  )
  .into()
}

#[proc_macro_attribute]
pub fn cmds_init(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let input_fn = parse_macro_input!(item as ItemFn);
  let ItemFn { sig, vis, block, attrs } = input_fn;

  let register_calls: Vec<_> = REGISTERED_COMMANDS
    .lock()
    .unwrap()
    .iter()
    .map(|name| {
      let register_name = format_ident!("__register_{}", name);
      quote! { #register_name(); }
    })
    .collect();

  quote!(
    #(#attrs)*
    #vis #sig {
      #(#register_calls)*
      #block
    }
  )
  .into()
}