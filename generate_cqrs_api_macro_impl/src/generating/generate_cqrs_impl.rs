use log::debug;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use stringcase::pascal_case_with_sep;
use stringcase::snake_case_with_sep;
use syn::Fields;
use syn::File;
use syn::Ident;
use syn::ImplItemFn;
use syn::Variant;

use crate::generate_api_macro_impl::ModelNEffectsNErrors;
use crate::parsing::type_2_ident::{get_ident, get_path, get_type_name};

pub(crate) fn generate_cqrs_impl(
    lifecycle_impl_ident: &Ident,
    models: &[ModelNEffectsNErrors],
) -> Vec<TokenStream> {
    models
        .iter()
        .map(|model| {
            let domain_model_ident = &model.domain_model_ident;
            let domain_model_lock_ident = &model.domain_model_lock_ident;
            let effect_ident = &model.effect_ident;
            let effect_variants = &model.effect_variants;
            let error_ident = &model.error_ident;

            let (cqrs_queries, cqrs_commands) = get_cqrs_functions(
                domain_model_ident,
                effect_ident,
                &model.error_ident,
                &model.ast,
            );

            let cqrs_queries_sig_tipes = get_cqrs_fns_sig_tipes(&cqrs_queries);
            let cqrs_commands_sig_tipes = get_cqrs_fns_sig_tipes(&cqrs_commands);
            let cqrs_queries_sig_idents = get_cqrs_fns_sig_idents(&cqrs_queries);
            let cqrs_commands_sig_idents = get_cqrs_fns_sig_idents(&cqrs_commands);

            let generated_cqrs_query_enum =
                generate_cqrs_query_enum(&cqrs_queries_sig_tipes, domain_model_ident);
            let generated_cqrs_command_enum =
                generate_cqrs_command_enum(&cqrs_commands_sig_tipes, domain_model_ident);

            let generated_cqrs_queries = generate_cqrs_functions(
                lifecycle_impl_ident,
                "Query",
                domain_model_ident,
                domain_model_lock_ident,
                &cqrs_queries_sig_idents,
                effect_ident,
                effect_variants,
                error_ident,
            );
            let generated_cqrs_commands = generate_cqrs_functions(
                lifecycle_impl_ident,
                "Command",
                domain_model_ident,
                domain_model_lock_ident,
                &cqrs_commands_sig_idents,
                effect_ident,
                effect_variants,
                error_ident,
            );

            quote! {
                #generated_cqrs_query_enum
                #generated_cqrs_command_enum
                #generated_cqrs_queries
                #generated_cqrs_commands
            }
        })
        .collect::<Vec<TokenStream>>()
}

fn generate_cqrs_functions(
    lifecycle_impl_ident: &Ident,
    cqrs_kind: &str,
    domain_model_struct_ident: &Ident,
    domain_model_lock_ident: &Ident,
    cqrs_queries_sig_idents: &[(Ident, Vec<Ident>)],
    effect: &Ident,
    effect_variants: &[Variant],
    processing_error: &Ident,
) -> TokenStream {
    let enum_ident = format_ident!("{}{}", domain_model_struct_ident, cqrs_kind);
    let domain_model_lock_var = format_ident!(
        "{}",
        snake_case_with_sep(&domain_model_lock_ident.to_string(), "_")
    );

    let lhs_cqrs_call = {
        let enum_variants = generate_cqrs_enum_variants(cqrs_queries_sig_idents);
        enum_variants.into_iter().map(|variant| {
            quote! {
                #enum_ident::#variant
            }
        })
    };

    let rhs_cqrs_call = cqrs_queries_sig_idents.iter().map(|(ident, args)| {
        let fn_call = format_ident!("{}", snake_case_with_sep(&ident.to_string(), "_"));
        quote! {
            #domain_model_lock_var. #fn_call ( #(#args),*)
        }
    });

    let effects_match_statements = effect_variants.iter().map(|variant| {
        // use the type as the ident, as enum variant payloads don't have a name
        let variant_fields_idents = match &variant.fields {
            Fields::Unit => {
                vec![]},
                _ => {
                    let variant_fields = match &variant.fields{
                        Fields::Unit => unreachable!(),
                        Fields::Named(fields_named) => &fields_named.named,
                        Fields::Unnamed(fields_unnamed) => &fields_unnamed.unnamed,
                    };
                    variant_fields.iter().map(|field| {
                        if let Some(field_name) = &field.ident {
                            field_name.to_owned()
                        } else {
                            get_type_name(&field.ty).expect("Couldn't get the types name!")
                        }
                    }
                    ).collect::<Vec<Ident>>()
                }
        };

        let lhs_ident = format_ident!("{}", variant.ident);
        let rhs_ident = format_ident!("{}{}", domain_model_struct_ident, variant.ident);

        if variant_fields_idents.is_empty() {
            quote! {
                #effect::#lhs_ident => Effect :: #rhs_ident,
            }
        } else {
            quote! {
                #effect::#lhs_ident ( #(#variant_fields_idents),* ) => Effect ::#rhs_ident ( #(#variant_fields_idents),* ),
            }
        }
    });

    let update_state_statement = if cqrs_kind == "Command" {
        quote! {
            if state_changed {
                app_state.mark_dirty();
                lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
            }
        }
    } else {
        quote! {}
    };

    let result_type = if cqrs_kind == "Command" {
        quote! {(state_changed, result)}
    } else {
        quote! {result}
    };

    // generate final code
    quote! {
        impl Cqrs for #enum_ident{
            fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                let lifecycle = #lifecycle_impl_ident::get_singleton();
                let app_state = &lifecycle.borrow_app_state();
                let #domain_model_lock_var = &app_state.#domain_model_lock_var;
                let #result_type = match self {
                    #(#lhs_cqrs_call => #rhs_cqrs_call,)*
                }
                .map_err(ProcessingError::#processing_error)?;
            #update_state_statement
            Ok(result
                .into_iter()
                .map(|effect| match effect {
                    #(#effects_match_statements)*
                })
                .collect())
            }
        }
    }
}

fn generate_cqrs_query_enum(
    cqrs_q_fns_sig_tipes: &[(Ident, Vec<Ident>)],
    domain_model_struct_ident: &Ident,
) -> TokenStream {
    generate_cqrs_enum(cqrs_q_fns_sig_tipes, "Query", domain_model_struct_ident)
}
fn generate_cqrs_command_enum(
    cqrs_q_fns_sig_tipes: &[(Ident, Vec<Ident>)],
    domain_model_struct_ident: &Ident,
) -> TokenStream {
    generate_cqrs_enum(cqrs_q_fns_sig_tipes, "Command", domain_model_struct_ident)
}
fn generate_cqrs_enum(
    cqrs_q_fns_sig_tipes: &[(Ident, Vec<Ident>)],
    cqrs_kind: &str,
    domain_model_struct_ident: &Ident,
) -> TokenStream {
    let enum_variants = generate_cqrs_enum_variants(cqrs_q_fns_sig_tipes);
    let cqrs_ident = format_ident!("{domain_model_struct_ident}{cqrs_kind}");
    let code = quote! {
        #[derive(Debug)]
        pub enum #cqrs_ident {
            #(#enum_variants),*
        }
    };

    debug!("\n\n{:#?}\n\n", code);
    code
}

fn generate_cqrs_enum_variants(cqrs_fns_sig_tipes: &[(Ident, Vec<Ident>)]) -> Vec<TokenStream> {
    cqrs_fns_sig_tipes
        .iter()
        .map(|(ident, args)| {
            // remove the prefix, if it is a variant of "command" or "query"
            let ident_string = ident.to_string();
            let cleaned_ident = if let Some(split_pos) = ident_string.find('_') {
                match &ident_string[0..split_pos] {
                    "command" | "com" | "query" => &ident_string[split_pos + 1..],
                    _ => ident_string.as_str(),
                }
            } else {
                ident_string.as_str()
            };
            let enum_variant = format_ident!("{}", pascal_case_with_sep(cleaned_ident, "_"));

            if args.is_empty() {
                quote! {#enum_variant}
            } else {
                quote! {
                    #enum_variant (#(#args),*)
                }
            }
        })
        .collect::<Vec<TokenStream>>()
}

/// extracts the signature of passed functions,
/// returning the types of the arguments
/// e.g.: foo(name: String, age: usize) => [foo [String, usize]]
fn get_cqrs_fns_sig_tipes(cqrs_fns: &[ImplItemFn]) -> Vec<(Ident, Vec<Ident>)> {
    cqrs_fns
        .iter()
        .map(|function| {
            let args_tipes = function
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    syn::FnArg::Typed(pat_type) => get_ident(&pat_type.ty).ok(),
                    syn::FnArg::Receiver(_) => None,
                })
                .collect::<Vec<Ident>>();
            (function.sig.ident.to_owned(), args_tipes)
        })
        .collect::<Vec<(Ident, Vec<Ident>)>>()
}
/// extracts the signature of passed functions,
/// returning the names of the arguments
/// e.g.: foo(name: String, age: usize) => [foo [name, age]]
fn get_cqrs_fns_sig_idents(cqrs_fns: &[ImplItemFn]) -> Vec<(Ident, Vec<Ident>)> {
    cqrs_fns
        .iter()
        .map(|function| {
            let args_tipes = function
                .sig
                .inputs
                .iter()
                .filter_map(|arg| match arg {
                    syn::FnArg::Typed(pat_type) => match *pat_type.pat.clone() {
                        syn::Pat::Ident(pat_ident) => Some(pat_ident.ident),
                        _ => None,
                    },
                    syn::FnArg::Receiver(_) => None,
                })
                .collect::<Vec<Ident>>();
            (function.sig.ident.to_owned(), args_tipes)
        })
        .collect::<Vec<(Ident, Vec<Ident>)>>()
}

/// @returns tuple (CQRS Queries, CQRS Commands)
fn get_cqrs_functions(
    domain_model_struct_ident: &Ident,
    effect: &Ident,
    processing_error: &Ident,
    ast: &File,
) -> (Vec<ImplItemFn>, Vec<ImplItemFn>) {
    let domain_model_lock_struct_ident = format_ident!("{domain_model_struct_ident}Lock");
    let mut cqrs_fns = ast
        .items
        .iter()
        .filter_map(|item| {
            // Filter for the lock struct
            match item {
                syn::Item::Impl(item_impl)
                    if get_ident(&item_impl.self_ty).ok()? == domain_model_lock_struct_ident =>
                {
                    Some(item_impl)
                }
                _ => None,
            }
        })
        // get all functions of the domain_model_lock struct
        .flat_map(|domain_model_lock_struckt| {
            domain_model_lock_struckt
                .items
                .iter()
                // get all functions
                .filter_map(|item| match item {
                    syn::ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
                    _ => None,
                })
                .collect::<Vec<&ImplItemFn>>()
        })
        // get all cqrs functions (Result<_, ProcessingError>)
        .filter_map(|function| {
            // get the return type
            let output_type = match &function.sig.output {
                syn::ReturnType::Type(_, tipe) => get_path(tipe).ok()?,
                _ => return None,
            };
            // filter for Result<_>
            let result_tipe = output_type.segments.last()?;
            if result_tipe.ident != "Result" {
                return None;
            }
            // get Result<inner_tipes>
            let mut inner_tipes = match &result_tipe.arguments {
                syn::PathArguments::AngleBracketed(arguments) => arguments.args.iter(),
                _ => return None,
            };
            // get Result<arg_pairs>, meaning the Result<left, right> content
            let left = inner_tipes.next()?;
            let right = inner_tipes.next()?;
            // filter out if there is something else!
            if inner_tipes.next().is_some() {
                return None;
            }
            // filter for Result<_, ProcessingError> (to get cqrs fns only)
            match right {
                syn::GenericArgument::Type(tipe) => match get_ident(tipe) {
                    Err(_) => None,
                    Ok(right_tipe) if right_tipe == *processing_error => {
                        Some((function, left.to_owned()))
                    }
                    Ok(_) => None,
                },
                _ => None,
            }
        })
        // Now we can focus on Result<left, _> only
        // iter.item is (filtered item_fn and left in the return type Result<left, ProcessingError>)
        // filter for cqrs_command: (StatusChanged, Vec<_>) or cqrs_query: Vec<_>
        .fold((vec![], vec![]), |mut acc, (impl_fn, left)| {
            // sort to (cqrs_command, cqrs_query), discard all others
            match left {
                syn::GenericArgument::Type(tipe) => {
                    match tipe {
                        // this should be a Vec<Effect>, indicating a CQRS Query
                        syn::Type::Path(type_path)
                            if match_vec_effect(&type_path.path.segments, effect) =>
                        {
                            acc.0.push(impl_fn.to_owned());
                            acc
                        }
                        // this should be a (StatusChanged, Vec<Effect>), indicating a CQRS Command
                        syn::Type::Tuple(type_tuple) => {
                            let mut type_tuple_iter = type_tuple.elems.iter();
                            let state_changed = type_tuple_iter.next();
                            let vec_effect = type_tuple_iter.next();
                            if type_tuple_iter.next().is_some() {
                                acc
                            } else if state_changed
                                .is_some_and(|tipe| get_ident(tipe).is_ok_and(|i| i == "bool"))
                                && vec_effect.is_some_and(|tipe| {
                                    get_path(tipe)
                                        .is_ok_and(|path| match_vec_effect(&path.segments, effect))
                                })
                            {
                                acc.1.push(impl_fn.to_owned());
                                acc
                            } else {
                                acc
                            }
                        }
                        _ => acc,
                    }
                }
                _ => acc,
            }
        });
    // sort the retrieved functions bu function name (=ident)
    cqrs_fns
        .0
        .sort_by_key(|function| function.sig.ident.clone());
    cqrs_fns
        .1
        .sort_by_key(|function| function.sig.ident.clone());
    if cqrs_fns.0.is_empty() && cqrs_fns.1.is_empty() {
        panic!(
            r#"Did not find a single cqrs-function! Be sure to implement them like:
            impl {domain_model_lock_struct_ident}{{
                fn my_cqrs_function(& self, OPTIONALLY_ANY_OTHER_PARAMETERS) -> Result<(bool, Vec<{effect}>), {processing_error}> {{...}}
            }}

            where 'bool' indicates if the state changed. If bool is present, we assume a CQRS-Command, otherwise a CQRS-Query.
            "#
        )
    }
    cqrs_fns
}

fn match_vec_effect(
    fn_to_match: &syn::punctuated::Punctuated<syn::PathSegment, syn::token::PathSep>,
    effect: &Ident,
) -> bool {
    if fn_to_match.len() != 1 {
        return false;
    }
    let Some(vec) = fn_to_match.first() else {
        return false;
    };

    if vec.ident != "Vec" {
        return false;
    }
    match vec.arguments.to_owned() {
        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
            match angle_bracketed_generic_arguments.args.first() {
                Some(syn::GenericArgument::Type(t)) => {
                    if let Ok(inner_tipe_ident) = get_ident(t) {
                        inner_tipe_ident == *effect
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {

    use quote::{format_ident, quote};
    use syn::{parse_str, Ident};

    use crate::{
        generate_api_macro_impl::{BasePath, ModelNEffectsNErrors},
        generating::generate_cqrs_impl::{
            generate_cqrs_command_enum, generate_cqrs_functions, generate_cqrs_impl,
            generate_cqrs_query_enum, get_cqrs_fns_sig_idents, get_cqrs_fns_sig_tipes,
            get_cqrs_functions,
        },
    };

    const CODE: &str = r#"
            impl NotMyModel {
                pub(crate) fn fake(
                    &self,
                    todo_pos: usize,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    let model_lock = Self::get_model_lock(app_state);
                    let items = &mut model_lock.blocking_write().items;
                    if todo_pos > items.len() {
                        Err(MyGoodProcessingError::ItemDoesNotExist(todo_pos))
                    } else {
                        items.remove(todo_pos - 1);
                        app_state.mark_dirty();
                        Ok(vec![MyGoodDomainModelEffect::RenderItems(
                            model_lock.clone(),
                        )])
                    }
                }
            }
            impl MyGoodDomainModelLock {
                pub(crate) fn wrong_fn_effect_type(
                    &self,
                    todo_pos: usize,
                ) -> Result<Vec<MyWrongEffect>, MyGoodProcessingError> {
                    let items = &mut self.lock.blocking_write().items;
                    if todo_pos > items.len() {
                        Err(MyGoodProcessingError::ItemDoesNotExist(todo_pos))
                    } else {
                        items.remove(todo_pos - 1);
                        Ok((true, vec![TodoListEffect::RenderTodoList(self.clone())]))
                    }
                }
                pub(crate) fn wrong_fn_error_type(
                    &self,
                    todo_pos: usize,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyWrongError> {
                    let model_lock = Self::get_model_lock(app_state);
                    let items = &mut model_lock.blocking_write().items;
                    if todo_pos > items.len() {
                        Err(MyGoodProcessingError::ItemDoesNotExist(todo_pos))
                    } else {
                        items.remove(todo_pos - 1);
                        app_state.mark_dirty();
                        Ok(vec![MyGoodDomainModelEffect::RenderItems(
                            model_lock.clone(),
                        )])
                    }
                }
                pub(crate) fn remove_item(
                    &self,
                    item_pos: usize,
                ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                    let items = &mut self.lock.blocking_write().items;
                    if item_pos > items.len() {
                        Err(MyGoodProcessingError::ItemDoesNotExist(item_pos))
                    } else {
                        items.remove(item_pos - 1);
                        Ok(true, vec![MyGoodDomainModelEffect::RenderItems(model_lock.clone())])
                    }
                }
            }
            impl MyGoodDomainModel {
                pub fn get_items_as_string(&self) -> Vec<String> {
                    self.items.iter().map(|item| item.text.clone()).collect()
                }

                pub(crate) fn add_item_2_model(
                    &self,
                    item: String,
                    priority: usize,
                ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                    self.lock
                    .blocking_write()
                    .items
                    .push(Item { text: item });
                    // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
                    Ok((true, vec![ItemListEffect::RenderItemList(self.clone())]))
                }
            }
            pub struct MyGoodDomainModelLock  {
                pub lock: RustAutoOpaque<MyGoodDomainMode>,
            }

            impl MyGoodDomainModelLock {
                pub fn get_items_as_string(&self) -> Vec<String> {
                    self.items.iter().map(|item| item.text.clone()).collect()
                }

                pub(crate) fn add_item(
                    &self,
                    item: String,
                    priority: usize,
                ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                    self.lock
                    .blocking_write()
                    .items
                    .push(DomainItem { text: item });
                    // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
                    Ok((true, vec![ItemListEffect::RenderItemList(self.clone())]))
                }
                pub(crate) fn command_clean_list(
                    &self,
                ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                    self.lock.blocking_write().items.clear();
                    Ok((true, vec![MyGoodDomainModelEffect::RenderItems(self.clone())]))
                }
                pub(crate) fn all_items(
                    &self,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    Ok(vec![MyGoodDomainModelEffect::RenderItems(self.clone())])
                }
                pub(crate) fn query_get_item(
                    &self,
                    item_pos: usize,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    let items = &self.lock.blocking_read().items;
                    if item_pos > items.len() {
                        Err(ItemListProcessingError::ItemDoesNotExist(item_pos))
                    } else {
                        let item = &items[item_pos - 1];
                        Ok(vec![MyGoodDomainModelEffect::RenderItems(item.clone())])
                   }
                }
            }
        "#;
    const CODE_SECOND_MODEL: &str = r#"
            impl MySecondDomainModelLock {
                pub(crate) fn copy_item(
                    &self,
                    item_pos: usize,
                ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondProcessingError> {
                    let items = &mut self.lock.blocking_write().items;
                    if item_pos > items.len() {
                        Err(MySecondProcessingError::ItemDoesNotExist(item_pos))
                    } else {
                        items.clone(item_pos - 1);
                        Ok(true, vec![MySecondDomainModelEffect::RenderItems(model_lock.clone())])
                    }
                }
            }
            impl MySecondDomainModel {
                pub fn get_items_as_string(&self) -> Vec<String> {
                    self.items.iter().map(|item| item.text.clone()).collect()
                }

                pub(crate) fn add_object_2_model(
                    &self,
                    object: String,
                    priority: usize,
                ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondProcessingError> {
                    self.lock
                    .blocking_write()
                    .items
                    .push(Item { text: item });
                    // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
                    Ok((true, vec![ItemListEffect::RenderItemList(self.clone())]))
                }
            }
            pub struct MySecondDomainModelLock  {
                pub lock: RustAutoOpaque<MySecondDomainMode>,
            }

            impl MySecondDomainModelLock {
                pub fn get_objects_as_string(&self) -> Vec<String> {
                    self.items.iter().map(|item| item.text.clone()).collect()
                }

                pub(crate) fn add_object(
                    &self,
                    item: String,
                    priority: usize,
                ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondProcessingError> {
                    self.lock
                    .blocking_write()
                    .items
                    .push(DomainItem { text: item });
                    // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
                    Ok((true, vec![ItemListEffect::RenderItemList(self.clone())]))
                }
                pub(crate) fn clean_all_objects(
                    &self,
                ) -> Result<(bool, Vec<MySecondDomainModelEffect>), MySecondProcessingError> {
                    self.lock.blocking_write().items.clear();
                    Ok((true, vec![MySecondDomainModelEffect::RenderItems(self.clone())]))
                }
                pub(crate) fn all_objects(
                    &self,
                ) -> Result<Vec<MySecondDomainModelEffect>, MySecondProcessingError> {
                    Ok(vec![MySecondDomainModelEffect::RenderItems(self.clone())])
                }
                pub(crate) fn query_get_object(
                    &self,
                    item_pos: usize,
                ) -> Result<Vec<MySecondDomainModelEffect>, MySecondProcessingError> {
                    let items = &self.lock.blocking_read().items;
                    if item_pos > items.len() {
                        Err(ItemListProcessingError::ItemDoesNotExist(item_pos))
                    } else {
                        let item = &items[item_pos - 1];
                        Ok(vec![MySecondDomainModelEffect::RenderItems(item.clone())])
                   }
                }
            }
        "#;

    #[test]
    fn get_cqrs_fns_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let (cqrs_queries, cqrs_commands) = get_cqrs_functions(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        let result = quote! {
            #(#cqrs_queries)*
            #(#cqrs_commands)*
        };

        let expected = quote! {
            pub (crate) fn all_items (&self ,) -> Result <  Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > {
                Ok(vec![MyGoodDomainModelEffect::RenderItems(self.clone())])
            }
            pub(crate) fn query_get_item(
                &self,
                item_pos: usize,
            ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                let items = &self.lock.blocking_read().items;
                if item_pos > items.len() {
                    Err(ItemListProcessingError::ItemDoesNotExist(item_pos))
                } else {
                    let item = &items[item_pos - 1];
                    Ok(vec![MyGoodDomainModelEffect::RenderItems(item.clone())])
                }
            }
            pub(crate) fn add_item(
                &self,
                item: String,
                priority: usize,
            ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                self.lock.blocking_write().items.push(DomainItem { text: item });
                Ok((true, vec![ItemListEffect::RenderItemList(self.clone())]))
            }
            pub(crate) fn command_clean_list(
                &self,
            ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                self.lock.blocking_write().items.clear();
                Ok((true, vec![MyGoodDomainModelEffect::RenderItems(self.clone())]))
            }
            pub(crate) fn remove_item(
                &self,
                item_pos: usize,
            ) -> Result<(bool, Vec<MyGoodDomainModelEffect>), MyGoodProcessingError> {
                let items = &mut self.lock.blocking_write().items;
                if item_pos > items.len() {
                    Err(MyGoodProcessingError::ItemDoesNotExist(item_pos))
                } else {
                    items.remove(item_pos - 1);
                    Ok( true, vec![MyGoodDomainModelEffect::RenderItems(model_lock.clone())] )
                }
            }
        };
        assert_eq!(expected.to_string(), result.to_string());
    }
    #[test]
    #[should_panic = "Did not find a single cqrs-function! Be sure to implement them like:
            impl SomeModelLock{
                fn my_cqrs_function(& self, OPTIONALLY_ANY_OTHER_PARAMETERS) -> Result<(bool, Vec<SomeModelEffect>), SomeModelError> {...}
            }

            where 'bool' indicates if the state changed. If bool is present, we assume a CQRS-Command, otherwise a CQRS-Query.
            "]
    fn get_cqrs_fns_fail_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let (cqrs_queries, cqrs_commands) = get_cqrs_functions(
            &format_ident!("SomeModel"),
            &format_ident!("SomeModelEffect"),
            &format_ident!("SomeModelError"),
            &ast,
        );
        let result = quote! {
            #(#cqrs_queries)*
            #(#cqrs_commands)*
        };

        let expected = "pub (crate) fn remove_item (app_state : & AppState , todo_pos : usize ,) -> Result < Vec < SomeModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; let items = & mut model_lock . blocking_write () . items ; if todo_pos > items . len () { Err (MyGoodProcessingError :: ItemDoesNotExist (todo_pos)) } else { items . remove (todo_pos - 1) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } } pub (crate) fn add_item (app_state : & AppState , item : String ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . push (DomainItem { text : item }) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn clean_list (& self ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . clear () ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn get_all_items (app_state : & AppState ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = MyGoodDomainModel :: get_model_lock (app_state) ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) }";
        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn print_found_fns() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");

        let result = get_cqrs_functions(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        assert_eq!(
            vec![
                "all_items",
                "query_get_item",
                "add_item",
                "command_clean_list",
                "remove_item"
            ],
            result
                .0
                .iter()
                .chain(&result.1)
                .map(|fns| fns.sig.ident.to_string())
                .collect::<Vec<String>>()
        );
    }

    #[test]
    fn generate_cqrs_enum_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let (cqrs_q, cqrs_c) = get_cqrs_functions(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        let cqrs_q_enum = generate_cqrs_query_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_q),
            &format_ident!("MyGoodDomainModel"),
        );
        let cqrs_c_enum = generate_cqrs_command_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_c),
            &format_ident!("MyGoodDomainModel"),
        );
        let result = quote! {
            #cqrs_q_enum
            #cqrs_c_enum
        };
        let expected = quote! {
            #[derive(Debug)]
            pub enum MyGoodDomainModelQuery {
                AllItems,
                GetItem(usize)
            }
            #[derive(Debug)]
            pub enum MyGoodDomainModelCommand  {
                AddItem(String, usize),
                CleanList,
                RemoveItem(usize)
            }
        };

        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn generate_cqrs_enum_test_two_models() {
        let ast = syn::parse_file(CODE).expect("test oracle model one should be parsable");
        let ast_2 =
            syn::parse_file(CODE_SECOND_MODEL).expect("test oracle model two should be parsable");
        let (cqrs_q, cqrs_c) = get_cqrs_functions(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        let cqrs_q_enum = generate_cqrs_query_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_q),
            &format_ident!("MyGoodDomainModel"),
        );
        let cqrs_c_enum = generate_cqrs_command_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_c),
            &format_ident!("MyGoodDomainModel"),
        );
        let (cqrs_q_2, cqrs_c_2) = get_cqrs_functions(
            &format_ident!("MySecondDomainModel"),
            &format_ident!("MySecondDomainModelEffect"),
            &format_ident!("MySecondProcessingError"),
            &ast_2,
        );
        let cqrs_q_enum_2 = generate_cqrs_query_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_q_2),
            &format_ident!("MySecondDomainModel"),
        );
        let cqrs_c_enum_2 = generate_cqrs_command_enum(
            &get_cqrs_fns_sig_tipes(&cqrs_c_2),
            &format_ident!("MySecondDomainModel"),
        );
        let result = quote! {
            #cqrs_q_enum
            #cqrs_c_enum
            #cqrs_q_enum_2
            #cqrs_c_enum_2
        };
        let expected = quote! {
            #[derive(Debug)]
            pub enum MyGoodDomainModelQuery {
                AllItems,
                GetItem(usize)
            }
            #[derive(Debug)]
            pub enum MyGoodDomainModelCommand  {
                AddItem(String, usize),
                CleanList,
                RemoveItem(usize)
            }
            #[derive(Debug)]
            pub enum MySecondDomainModelQuery {
                AllObjects,
                GetObject(usize)
            }
            #[derive(Debug)]
            pub enum MySecondDomainModelCommand {
                AddObject(String, usize),
                CleanAllObjects,
                CopyItem(usize)
            }
        };
        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn generate_cqrs_fns_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let domain_model_struct_ident = format_ident!("MyGoodDomainModel");
        let domain_model_lock_ident = format_ident!("MyGoodDomainModelLock");
        let processing_error = format_ident!("MyGoodProcessingError");
        let effect_code = parse_str::<syn::ItemEnum>(
            r#"pub enum MyGoodDomainModelEffect {
            RenderItemList(model_lock),
            RenderItem(item),
            RenderMyGoodDomainModel(model_lock) 
            }
            "#,
        )
        .expect("Couldn't parse test oracle!");
        let effect_ident = effect_code.ident;
        let (cqrs_q, cqrs_c) = get_cqrs_functions(
            &domain_model_struct_ident,
            &effect_ident,
            &processing_error,
            &ast,
        );
        let effect_variants: Vec<syn::Variant> = effect_code
            .variants
            .into_pairs()
            .map(|pair| pair.value().clone())
            .collect();
        let lifecycle_impl_ident: Ident = format_ident!("LifecycleImpl");
        let cqrs_queries = generate_cqrs_functions(
            &lifecycle_impl_ident,
            "Query",
            &domain_model_struct_ident,
            &domain_model_lock_ident,
            &get_cqrs_fns_sig_idents(&cqrs_q),
            &effect_ident,
            &effect_variants,
            &processing_error,
        );
        let cqrs_commands = generate_cqrs_functions(
            &lifecycle_impl_ident,
            "Command",
            &domain_model_struct_ident,
            &domain_model_lock_ident,
            &get_cqrs_fns_sig_idents(&cqrs_c),
            &effect_ident,
            &effect_variants,
            &processing_error,
        );
        let result = quote! {
           #cqrs_queries
           #cqrs_commands
        };

        let expected = quote! {
            impl Cqrs for MyGoodDomainModelQuery {
                fn process (self) -> Result < Vec < Effect > , ProcessingError > {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state = &lifecycle.borrow_app_state();
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let result = match self {
                        MyGoodDomainModelQuery::AllItems => my_good_domain_model_lock.all_items(),
                        MyGoodDomainModelQuery::GetItem(item_pos) => my_good_domain_model_lock.query_get_item(item_pos),
                    }.map_err(ProcessingError::MyGoodProcessingError)?;
                    Ok(result
                        .into_iter()
                        .map(|effect| match effect {
                            MyGoodDomainModelEffect::RenderItemList(model_lock) =>
                                Effect::MyGoodDomainModelRenderItemList(model_lock),
                            MyGoodDomainModelEffect::RenderItem(item) =>
                                Effect::MyGoodDomainModelRenderItem(item),
                            MyGoodDomainModelEffect::RenderMyGoodDomainModel(model_lock) =>
                                Effect::MyGoodDomainModelRenderMyGoodDomainModel(model_lock)
                        , })
                        .collect())
                    }
                }
            impl Cqrs for MyGoodDomainModelCommand {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state = &lifecycle.borrow_app_state();
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let (state_changed, result) = match self {
                        MyGoodDomainModelCommand::AddItem(item, priority) => my_good_domain_model_lock.add_item(item, priority),
                        MyGoodDomainModelCommand::CleanList => my_good_domain_model_lock.command_clean_list(),
                        MyGoodDomainModelCommand::RemoveItem(item_pos) => my_good_domain_model_lock.remove_item(item_pos) ,
                    }
                    .map_err(ProcessingError::MyGoodProcessingError)?;
                    if state_changed {
                        app_state.mark_dirty();
                        lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
                    }
                    Ok(result
                    .into_iter()
                    .map(|effect| match effect {
                        MyGoodDomainModelEffect::RenderItemList(model_lock) =>
                        Effect::MyGoodDomainModelRenderItemList(model_lock),
                        MyGoodDomainModelEffect::RenderItem(item) => Effect::MyGoodDomainModelRenderItem(item),
                        MyGoodDomainModelEffect::RenderMyGoodDomainModel(model_lock) => Effect::MyGoodDomainModelRenderMyGoodDomainModel(model_lock)
                    , })
                    .collect())
                }
            }
        };

        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn generate_cqrs_impl_test_two_models() {
        let ast = syn::parse_file(CODE).expect("test oracle for model one should be parsable");
        let ast_2 = syn::parse_file(CODE_SECOND_MODEL)
            .expect("test oracle for model two should be parsable");

        let effect_code = parse_str::<syn::ItemEnum>(
            r#"pub enum MyGoodDomainModelEffect {
                RenderItemList(model_lock),
                RenderItem(item),
                RenderMyGoodDomainModel(model_lock) 
            }
            "#,
        )
        .expect("Couldn't parse test oracle!");
        let effect_code_2 = parse_str::<syn::ItemEnum>(
            r#"pub enum MySecondDomainModelEffect {
                RenderItems(model_lock),
                RenderItem(item),
                RenderMySecondDomainModel(model_lock) 
            }
            "#,
        )
        .expect("Couldn't parse test oracle!");
        let effect_variants: Vec<syn::Variant> = effect_code
            .variants
            .into_pairs()
            .map(|pair| pair.value().clone())
            .collect();
        let effect_variants_2: Vec<syn::Variant> = effect_code_2
            .variants
            .into_pairs()
            .map(|pair| pair.value().clone())
            .collect();

        let models = vec![
            ModelNEffectsNErrors {
                base_path: BasePath("domain::model".to_string()),
                ast,
                domain_model_ident: format_ident!("MyGoodDomainModel"),
                domain_model_lock_ident: format_ident!("MyGoodDomainModelLock"),
                effect_ident: effect_code.ident,
                effect_variants,
                error_ident: format_ident!("MyGoodProcessingError"),
            },
            ModelNEffectsNErrors {
                base_path: BasePath("domain::other".to_string()),
                ast: ast_2,
                domain_model_ident: format_ident!("MySecondDomainModel"),
                domain_model_lock_ident: format_ident!("MySecondDomainModelLock"),
                effect_ident: effect_code_2.ident,
                effect_variants: effect_variants_2,
                error_ident: format_ident!("MySecondProcessingError"),
            },
        ];
        let lifecycle_impl_ident: Ident = format_ident!("LifecycleImpl");
        let generated_cqrs = generate_cqrs_impl(&lifecycle_impl_ident, &models);
        let result = quote! {
            #(#generated_cqrs)*
        };

        let expected = quote! {
            #[derive(Debug)]
            pub enum MyGoodDomainModelQuery {
                AllItems,
                GetItem(usize)
            }
            #[derive(Debug)]
            pub enum MyGoodDomainModelCommand {
                AddItem(String, usize),
                CleanList,
                RemoveItem(usize)
            }
            impl Cqrs for MyGoodDomainModelQuery {
                fn process (self) -> Result < Vec < Effect > , ProcessingError > {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state = &lifecycle.borrow_app_state();
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let result = match self {
                        MyGoodDomainModelQuery::AllItems => my_good_domain_model_lock.all_items(),
                        MyGoodDomainModelQuery::GetItem(item_pos) => my_good_domain_model_lock.query_get_item(item_pos),
                    }.map_err(ProcessingError::MyGoodProcessingError)?;
                    Ok(result
                        .into_iter()
                        .map(|effect| match effect {
                            MyGoodDomainModelEffect::RenderItemList(model_lock) =>
                                Effect::MyGoodDomainModelRenderItemList(model_lock),
                            MyGoodDomainModelEffect::RenderItem(item) =>
                                Effect::MyGoodDomainModelRenderItem(item),
                            MyGoodDomainModelEffect::RenderMyGoodDomainModel(model_lock) =>
                                Effect::MyGoodDomainModelRenderMyGoodDomainModel(model_lock)
                        , })
                        .collect())
                    }
                }
            impl Cqrs for MyGoodDomainModelCommand {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state = &lifecycle.borrow_app_state();
                    let my_good_domain_model_lock = &app_state.my_good_domain_model_lock;
                    let (state_changed, result) = match self {
                        MyGoodDomainModelCommand::AddItem(item, priority) => my_good_domain_model_lock.add_item(item, priority),
                        MyGoodDomainModelCommand::CleanList => my_good_domain_model_lock.command_clean_list(),
                        MyGoodDomainModelCommand::RemoveItem(item_pos) => my_good_domain_model_lock.remove_item(item_pos) ,
                    }
                    .map_err(ProcessingError::MyGoodProcessingError)?;
                    if state_changed {
                        app_state.mark_dirty();
                        lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
                    }
                    Ok(result
                    .into_iter()
                    .map(|effect| match effect {
                        MyGoodDomainModelEffect::RenderItemList(model_lock) =>
                        Effect::MyGoodDomainModelRenderItemList(model_lock),
                        MyGoodDomainModelEffect::RenderItem(item) => Effect::MyGoodDomainModelRenderItem(item),
                        MyGoodDomainModelEffect::RenderMyGoodDomainModel(model_lock) => Effect::MyGoodDomainModelRenderMyGoodDomainModel(model_lock)
                    , })
                    .collect())
                }
            }
            #[derive(Debug)]
            pub enum MySecondDomainModelQuery {
                AllObjects,
                GetObject(usize)
            }
            #[derive(Debug)]
            pub enum MySecondDomainModelCommand {
                AddObject(String, usize),
                CleanAllObjects,
                CopyItem(usize)
            }
            impl Cqrs for MySecondDomainModelQuery {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state  = &lifecycle.borrow_app_state();
                    let my_second_domain_model_lock = &app_state.my_second_domain_model_lock;
                    let result = match self {
                        MySecondDomainModelQuery::AllObjects => my_second_domain_model_lock.all_objects(),
                        MySecondDomainModelQuery::GetObject(item_pos) => my_second_domain_model_lock.query_get_object(item_pos),
                    }
                    .map_err(ProcessingError::MySecondProcessingError)?;
                    Ok(result
                        .into_iter()
                        .map(|effect| match effect {
                            MySecondDomainModelEffect::RenderItems(model_lock) => Effect::MySecondDomainModelRenderItems(model_lock),
                            MySecondDomainModelEffect::RenderItem(item) => Effect::MySecondDomainModelRenderItem(item),
                            MySecondDomainModelEffect::RenderMySecondDomainModel(model_lock) => Effect::MySecondDomainModelRenderMySecondDomainModel(model_lock),
                        })
                        .collect())
                }
            }
            impl Cqrs for MySecondDomainModelCommand {
                fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                    let lifecycle = LifecycleImpl::get_singleton();
                    let app_state = &lifecycle.borrow_app_state();
                    let my_second_domain_model_lock = &app_state.my_second_domain_model_lock;
                    let (state_changed, result) = match self {
                        MySecondDomainModelCommand::AddObject(item, priority) => my_second_domain_model_lock.add_object(item, priority),
                        MySecondDomainModelCommand::CleanAllObjects => my_second_domain_model_lock.clean_all_objects(),
                        MySecondDomainModelCommand::CopyItem(item_pos) => my_second_domain_model_lock.copy_item(item_pos),
                    }
                    .map_err(ProcessingError::MySecondProcessingError)?;
                    if state_changed {
                        app_state.mark_dirty();
                        lifecycle.persist().map_err(ProcessingError::NotPersisted)?;
                    }
                    Ok(result
                        .into_iter()
                        .map(|effect| match effect {
                            MySecondDomainModelEffect::RenderItems(model_lock) => Effect::MySecondDomainModelRenderItems(model_lock),
                            MySecondDomainModelEffect::RenderItem(item) => Effect::MySecondDomainModelRenderItem(item),
                            MySecondDomainModelEffect::RenderMySecondDomainModel(model_lock) => Effect::MySecondDomainModelRenderMySecondDomainModel(model_lock),
                        })
                    .collect())
                }
            }
        };

        assert_eq!(expected.to_string(), result.to_string());
    }
}
