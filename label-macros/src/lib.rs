// Let's hope this is stabilized soon. There is some activity on it. https://github.com/rust-lang/rust/issues/54725
#![feature(proc_macro_span)]
//! # Label
//!
//! `label` is a library that can be used to create custom attributes for functions, through which you can list them and perform actions on them.
//!
//! For more documentation, refer to [https://docs.rs/label](https://docs.rs/label).
//!

extern crate proc_macro;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::export::ToTokens;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

struct ParsableAttribute {
    pub attributes: Vec<syn::Attribute>,
}

impl Parse for ParsableAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(ParsableAttribute {
            attributes: input.call(syn::Attribute::parse_outer)?,
        })
    }
}

fn simplify_path(mut path: syn::Path) -> syn::Path {
    // replace ::annotate with ::add in the path.
    // It would be cleaner to remove ::annotate entirely, but couldn't find
    // a way to do that. .pop() retains the ::.
    if let Some(i) = path.segments.last_mut() {
        assert_eq!(i.ident.to_string(), "label");
        let new_ident = syn::Ident::new("add", i.span());
        i.ident = new_ident;
    }

    path
}

enum Item {
    Func(syn::ItemFn),
    Static(syn::ItemStatic),
    Const(syn::ItemConst),
}

impl Item {
    pub fn name(&self) -> &syn::Ident {
        match self {
            Item::Func(i) => &i.sig.ident,
            Item::Static(i) => &i.ident,
            Item::Const(i) => &i.ident,
        }
    }

    pub fn attrs(&self) -> Vec<syn::Attribute> {
        match self {
            Item::Func(i) => i.attrs.clone(),
            Item::Static(i) => i.attrs.clone(),
            Item::Const(i) => i.attrs.clone(),
        }
    }

    pub fn set_attrs(&mut self, attrs: Vec<syn::Attribute>) {
        match self {
            Item::Func(i) => i.attrs = attrs,
            Item::Static(i) => i.attrs = attrs,
            Item::Const(i) => i.attrs = attrs,
        }
    }
}

impl ToTokens for Item {
    fn to_tokens(&self, tokens: &mut syn::export::TokenStream2) {
        match self {
            Item::Func(i) => i.to_tokens(tokens),
            Item::Static(i) => i.to_tokens(tokens),
            Item::Const(i) => i.to_tokens(tokens),
        }
    }
}

impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        let tokens = input.fork();
        if let Ok(i) = tokens.parse() {
            input.advance_to(&tokens);
            return Ok(Item::Func(i));
        }

        let tokens = input.fork();
        if let Ok(i) = tokens.parse() {
            input.advance_to(&tokens);
            return Ok(Item::Static(i));
        }

        let tokens = input.fork();
        if let Ok(i) = tokens.parse() {
            input.advance_to(&tokens);
            return Ok(Item::Const(i));
        }

        Err(input.error("Expected either function definition, static variable or const variable."))
    }
}

#[proc_macro_attribute]
#[doc(hidden)]
/// DO NOT USE DIRECTLY! USE THROUGH CREATE_ANNOTATION
pub fn __label(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(item as Item);

    // other annotation attributes
    let mut other_annotations = Vec::new();
    // any other attribute present
    let mut other_attrs = Vec::new();
    for i in item.attrs() {
        if let Some(ref lst) = i.path.segments.last() {
            if &*lst.ident.to_string() == "label" {
                other_annotations.push(simplify_path(i.path));
                continue;
            }
        }
        other_attrs.push(i);
    }

    // remove all label from the function's attributes
    // but keep other attributes
    item.set_attrs(other_attrs);

    let item_name = item.name();

    // for the following, the feature
    // #![feature(proc_macro_quote)]
    // could be used together with syn and proc_macro::quote_span.
    // However, this feature does not look like it will stabilize any time soon.
    // Therefore a regex is in my opinion currently better suited.
    let spans = {
        let mut current = Span::call_site();
        let mut possible = vec![current];

        while let Some(new) = current.parent() {
            current = new;
            possible.push(new);
        }

        possible
    };

    let mut res = None;
    for span in spans
        .iter()
        .rev()
        .map(|i| i.source_text())
        .filter_map(|i| i)
    {
        if let Ok(i) = syn::parse_str::<ParsableAttribute>(&span) {
            res = Some(i);
            break;
        }
    }

    let path: syn::Path = if let Some(res) = res {
        res.attributes[0].path.clone()
    } else {
        unreachable!()
    };

    let callpath = simplify_path(path);
    let item_name_str = format!("{}", item_name);

    let item_quote = match &item {
        Item::Func(_) => quote! {
            #item_name
        },
        Item::Static(_) => {
            quote! {
                &#item_name
            }
        }
        Item::Const(_) => quote! {
            &#item_name
        },
    };

    let result = quote! {
        #item

        #[allow(non_snake_case)]
        // This uses: https://github.com/rust-lang/rust/issues/54912 to make anonymous modules.
        // Anonymous modules use the parent scope meaning no more imports of `super::*` are needed
        const _: () = {
            use label::ctor;

            #[ctor]
            fn create () {
                // Safety: This is unsafe because sometimes I use mut statics here. However, I'm only giving out pointers
                // to them for which I make sure you can't use them without an unsafe block where they are used.
                unsafe {
                    // register for all label it should be registered for
                    #callpath::__add_label(#item_name_str, #item_quote);

                    #(#other_annotations ::__add_label(#item_name_str, #item_quote);)*
                }
            }
        };
    };

    result.into()
}

struct Definitions {
    signatures: Punctuated<Definition, syn::Token![;]>,
}

impl Parse for Definitions {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            signatures: input.parse_terminated::<_, syn::Token![;]>(Definition::parse)?,
        })
    }
}

enum Definition {
    Function {
        name: syn::Ident,
        params: syn::punctuated::Punctuated<syn::BareFnArg, syn::Token![,]>,
        generics: syn::Generics,
        returntype: syn::ReturnType,
    },
    Static {
        name: syn::Ident,
        var_type: syn::Type,
    },
}

impl Parse for Definition {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<syn::Visibility>();

        if input.peek(syn::Token![fn]) {
            input.parse::<syn::Token![fn]>()?;

            let name = input.parse::<syn::Ident>()?;
            let before = input.fork();

            let generics: syn::Generics = input.parse()?;

            if generics.type_params().next().is_some() {
                return Err(
                    before.error("Labels can not have generic type parameters (only lifetimes).")
                );
            }
            if generics.const_params().next().is_some() {
                return Err(before.error("Labels can not have const parameters (only lifetimes)."));
            }

            let content;
            syn::parenthesized!(
               content in input
            );

            let params = content.parse_terminated::<_, syn::Token![,]>(syn::BareFnArg::parse)?;

            let returntype = input.parse::<syn::ReturnType>()?;

            Ok(Definition::Function {
                name,
                params,
                generics,
                returntype,
            })
        } else if input.peek(syn::Token![static]) {
            input.parse::<syn::Token![static]>()?;

            let _ = input.parse::<syn::Token![mut]>();
            let name = input.parse::<syn::Ident>()?;

            input.parse::<syn::Token![:]>()?;

            let var_type: syn::Type = input.parse()?;

            Ok(Definition::Static { name, var_type })
        } else if input.peek(syn::Token![const]) {
            input.parse::<syn::Token![const]>()?;

            let name = input.parse::<syn::Ident>()?;

            input.parse::<syn::Token![:]>()?;

            let var_type: syn::Type = input.parse()?;

            Ok(Definition::Static { name, var_type })
        } else {
            Err(input
                .error("Expected either function definition, static variable or const variable."))
        }
    }
}

#[proc_macro]
/// Creates a new label.
/// ```
/// create_label!(fn test() -> ());
/// ```
///
/// To use a label, add an attribute to a function in the following style:
///
/// ```
/// #[test::label]
/// fn my_function() {
///     // contents
/// }
///
/// ```
///
/// `test` is the name of your label (this has to be a full path to it. Labels can be imported).
/// The annotation has to end with `::label`, or otherwise it will not compile.
///
///
/// It is possible to create multipe labels in one invocation of the `create_label!()` macro. The syntax for this is as follows:
/// ```
/// create_label!(
///     fn test() -> ();
///     fn test1(usize) -> (usize);
///     fn test2(usize) -> (isize);
/// );
///
/// ```
///
/// It is not supported to have two labels in scope with the same name, just like two structs in the same scope with the same name won't work either.
///
///1
/// After a label is created, it is possible to iterate over all functions annotated with this label, using the iter function:
///
/// ```
/// for func in test::iter() {
///     // do something with the function
///     func();
/// }
///
/// ```
///
/// The order in which iteration occurs is *not* defined.
///
/// Alternatively, you can iterate over functions and their names using the `iter_named()` function:
///
/// ```
/// for (name, func) in test::iter_named() {
///     println!("name: {}", name);
///
///     // do something with the function
///     func();
/// }
///
/// ```
///
/// Labels can also be given to `static` or `const` variables. Iterating over such labeled variables
/// returns an `&'static` reference to the variable. You can define variable labels with
/// `create_label!()`. It does not matter if you use `const` or `static`, they are handled the same.
///  `static mut` is supported, though iterating over labels will *never* allow you to mutate these
///  variables. `static mut` in `create_label!()` does nothing. If a `static mut` is locally updated,
///  and the label is iterated over, the changed value is reflected.
///
/// ```
/// create_label!(
///     const name: usize;
///     static other_name: usize;
/// );
/// ```
///
/// ```
/// for i in name::iter() {
///     println!("value: {}", *i);
/// }
/// ```
///
///
pub fn create_label(signatures: TokenStream) -> TokenStream {
    let labels = syn::parse_macro_input!(signatures as Definitions)
        .signatures
        .iter()
        .map(|definition| {
            let (signature, name) = match definition {
                Definition::Function {
                    name,
                    generics,
                    params,
                    returntype,
                } => {
                    let lifetimes = generics.lifetimes();

                    (
                        quote! {
                            for <#(#lifetimes),*> fn(#params) #returntype
                        },
                        name,
                    )
                }
                Definition::Static { name, var_type } => (
                    quote! {
                        &'static #var_type
                    },
                    name,
                ),
            };

            quote! {
                #[allow(non_snake_case)]
                pub mod #name {
                    use super::*;

                    pub use std::collections::HashMap;
                    pub use label::__label as label;

                    pub static mut FUNCTIONS: Option<Vec<(&'static str, #signature)>> = None;

                    pub fn iter() -> impl Iterator<Item = #signature> {
                        // Safety: after FUNCTIONS is populated (before main is called),
                        // FUNCTIONS remains unchanged for the entire rest of the program.

                        unsafe{
                            FUNCTIONS.iter().flat_map(|i| i.iter().map(|i| &i.1)).cloned()
                        }
                    }

                    pub fn iter_named() -> impl Iterator<Item = (&'static str, #signature)> {
                        // Safety: after FUNCTIONS is populated (before main is called),
                        // FUNCTIONS remains unchanged for the entire rest of the program.
                        unsafe{
                            FUNCTIONS.iter().flat_map(|i| i).cloned()
                        }
                    }

                    pub mod add {
                        use super::*;
                        // WARNING: DO NOT CALL. THIS HAS TO BE PUBLIC FOR OTHER
                        // PARTS OF THE LIBRARY TO WORK BUT SHOULD NEVER BE USED.
                        pub fn __add_label(name: &'static str, func: #signature) {
                            unsafe {
                                if let Some(f) = &mut FUNCTIONS {
                                    f.push((name, func));
                                } else {
                                    FUNCTIONS = Some(vec![(name, func)])
                                }
                            }
                        }
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    let result = quote! {
        #(#labels)*
    };

    result.into()
}
