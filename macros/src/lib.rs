use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::sync::Mutex;
use syn::{parse_macro_input, ItemFn, Meta};

// A static mutable global map to keep track of registered commands
static REGISTERED_COMMANDS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[proc_macro_attribute]
pub fn shell_cmd(attr: TokenStream, item: TokenStream) -> TokenStream {
  let attr = parse_macro_input!(attr as Meta);
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

  let cmds_ident = match attr {
    Meta::Path(path) => path.get_ident().unwrap().clone(),
    _ => panic!("Expected a single identifier as attribute argument"),
  };

  let register_function_name = format_ident!("__register_{}", function_name);

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
      #cmds_ident.lock().unwrap().insert(#function_name, #function_identifier);
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
      info!("register all commands");

      #(#register_statements)*

      let __result = {
        #(#statements)*
      };

      return __result;
    }
  )
  .into()
}
