use std::f64::consts::E;
use std::iter;

use log::debug;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use stringcase::pascal_case_with_sep;
use syn::File;
use syn::FnArg;
use syn::Ident;
use syn::ImplItemFn;
use syn::Item;
use syn::ItemImpl;
use syn::PatType;
use syn::Type;
use syn::Variant;

use crate::utils::get_enum::get_enum_by_ident_keyword;

use super::get_domain_model_struct::get_type_ident;
use super::get_domain_model_struct::get_type_path;

pub(crate) fn generate_cqrs_impl(
    domain_model_struct_ident: &Ident,
    effect: &Ident,
    effect_variants: &Vec<Variant>,
    processing_error: &Ident,
    ast: &File,
) -> TokenStream {
    let cqrs_functions =
        get_cqrs_functions(domain_model_struct_ident, effect, processing_error, ast);

    let cqrs_functions_sig = get_cqrs_fns_sig(cqrs_functions);
    let cqrs_functions_sig_tipes = get_cqrs_fns_sig_tipes(&cqrs_functions_sig);
    let cqrs_functions_sig_idents = get_cqrs_fns_sig_idents(&cqrs_functions_sig);

    let generated_cqrs_enum =
        generate_cqrs_enum(&cqrs_functions_sig_tipes, domain_model_struct_ident);
    let generated_cqrs_fns = generate_cqrs_functions(
        &cqrs_functions_sig_idents,
        domain_model_struct_ident,
        effect,
        effect_variants,
        processing_error,
    );
    quote! {
        #generated_cqrs_enum
        #generated_cqrs_fns
    }
}

fn generate_cqrs_functions(
    cqrs_fns_sig_idents: &Vec<(Ident, Vec<Ident>)>,
    domain_model_struct_ident: &Ident,
    effect: &Ident,
    effect_variants: &Vec<Variant>,
    processing_error: &Ident,
) -> TokenStream {
    // let lhs_sig = cqrs_fns_sig.iter()
    //    .map(|(tipe, agrs)| {
    //         let (lhs, rhs) = sig;
    let functions = cqrs_fns_sig_idents.iter().map(|(ident, args)| {
        let fn_ident = format_ident!(
            "{}{}",
            domain_model_struct_ident,
            pascal_case_with_sep(&ident.to_string(), "_")
        );

        //remove 'app_statement' from the lhs arguments
        let lhs_args =
        if args[0] == "app_state" {
            args[1..].to_vec()
        } else {
            panic!("'impl Cqrs {{' functions have to have 'app_state: AppState' as the first argument! Found {} instead.",args[0]);
        };

        let lhs = if lhs_args.is_empty() {
            quote! {Cqrs::#fn_ident}
        } else {
            quote! {
                Cqrs::#fn_ident (#(#lhs_args), *)
            }
        };
        (lhs, args, ident)
    });

    let match_statement = functions.map(|(lhs, args, rhs_ident)| {
        quote! {
            #lhs => #domain_model_struct_ident::#rhs_ident(#(#args),*),
        }
    });

    let effects_match_statements = effect_variants.iter().map(|variant| {
        // use the type as the ident, as enum variant payloads don't have a name
        let variant_field_names_lhs = variant.fields.iter().map(|field| {
            format_ident!("{}",
            stringcase::snake_case(
                &get_type_ident(&field.ty)
                    .expect("Tipe of effect enum content exists!")
                    .to_string(),
            ))
        });
        let variant_field_names_rhs = variant_field_names_lhs.clone();
        let lhs_ident = format_ident!("{}", variant.ident);
        let rhs_ident = format_ident!("{}{}", domain_model_struct_ident, variant.ident);
        
        if variant.fields.is_empty() {
            quote! {
                #effect::#lhs_ident => Effect :: #rhs_ident,
            }
        } else {
            quote! {
                #effect::#lhs_ident ( #(#variant_field_names_lhs),* ) => Effect ::#rhs_ident ( #(#variant_field_names_rhs),* ),
            }
        }
    });

    quote! {
        impl Cqrs {
            pub(crate) fn process_with_app_state(
                self,
                app_state: &AppState,
            ) -> Result<Vec<Effect>, ProcessingError> {
                let result = match self {
                    //
                    #(#match_statement)*
                    // Cqrs::TodoCommandAddTodo(todo) => #domain_model_struct_ident::add_todo(app_state, todo),
                    // Cqrs::TodoCommandRemoveTodo(todo_pos) =>
                    //     #domain_model_struct_ident::remove_todo(app_state, todo_pos),
                    // Cqrs::TodoCommandCleanList => #domain_model_struct_ident::clean_list(app_state),
                    // Cqrs::TodoQueryAllTodos => #domain_model_struct_ident::get_all_todos(app_state),
                }//
                .map_err(ProcessingError::#processing_error)?
                .into_iter()
                .map(|effect| match effect {
                    #(#effects_match_statements)*
                    // TodoListEffect::RenderTodoList(content) => {
                    //     #effect::TodoListEffectRenderTodoList(content)
                    // }
                })
                .collect();
                Ok(result)
            }
            pub fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                let app_state = &Lifecycle::get().app_state;
                let result = self.process_with_app_state(app_state)?;
                //persist the state, but only if dirty
                let _ = app_state.persist().map_err(ProcessingError::NotPersisted);
                Ok(result)
            }
        }
    }
}
fn generate_cqrs_enum(
    cqrs_fns_sig_tipes: &Vec<(Ident, Vec<Ident>)>,
    domain_model_struct_ident: &Ident,
) -> TokenStream {
    let enum_variants = cqrs_fns_sig_tipes.iter().map(|(ident, args)| {
        let composed_fn_ident = format_ident!(
            "{}{}",
            domain_model_struct_ident,
            pascal_case_with_sep(&ident.to_string(), "_")
        );

        if args.is_empty() {
            quote! {#composed_fn_ident}
        } else {
            quote! {
                #composed_fn_ident (#(#args),*)
            }
        }
    });

    let code = quote! {
        pub enum Cqrs {
            #(#enum_variants),*
        }
    };

    debug!("\n\n{:#?}\n\n", code);
    code
}

/// extracts the signature of passed functions,
/// returning the types of the arguments
/// e.g.: foo(name: String, age: usize) => [foo [String, usize]]
fn get_cqrs_fns_sig_tipes(cqrs_fns_sig: &Vec<(Ident, Vec<PatType>)>) -> Vec<(Ident, Vec<Ident>)> {
    cqrs_fns_sig
        .iter()
        .map(|(ident, args)| {
            let args_tipes = args
                .iter()
                .filter_map(|arg| get_type_ident(&arg.ty).ok())
                .collect::<Vec<Ident>>();
            (ident.to_owned(), args_tipes)
        })
        .collect::<Vec<(Ident, Vec<Ident>)>>()
}
/// extracts the signature of passed functions,
/// returning the names of the arguments
/// e.g.: foo(name: String, age: usize) => [foo [name, age]]
fn get_cqrs_fns_sig_idents(cqrs_fns_sig: &Vec<(Ident, Vec<PatType>)>) -> Vec<(Ident, Vec<Ident>)> {
    cqrs_fns_sig
        .iter()
        .map(|(ident, args)| {
            let args_tipes = args
                .iter()
                .filter_map(|arg| match &*arg.pat {
                    syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                    _ => None,
                })
                .collect::<Vec<Ident>>();
            (ident.to_owned(), args_tipes)
        })
        .collect::<Vec<(Ident, Vec<Ident>)>>()
}

fn get_cqrs_fns_sig(cqrs_functions: Vec<ImplItemFn>) -> Vec<(Ident, Vec<PatType>)> {
    let cqrs_functions_sig = cqrs_functions
        .iter()
        .map(|function| {
            let fn_ident = function.sig.ident.to_owned();
            let arg_types = function
                .sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let syn::FnArg::Typed(arg) = arg {
                        Some(arg.to_owned())
                    } else {
                        None
                    }
                })
                .collect::<Vec<PatType>>();
            (fn_ident, arg_types)
        })
        .collect::<Vec<(Ident, Vec<PatType>)>>();
    cqrs_functions_sig
}

fn get_cqrs_functions(
    domain_model_struct_ident: &Ident,
    effect: &Ident,
    processing_error: &Ident,
    ast: &File,
) -> Vec<ImplItemFn> {
    let cqrs_fns = ast
        .items
        .iter()
        .filter_map(|item| {
            // Filter for non-trait implementations of domain_struct_ident
            let item_impl = match item {
                syn::Item::Impl(item_impl) if item_impl.trait_.is_none() => item_impl,
                _ => return None,
            };
            if get_type_ident(&item_impl.self_ty).ok()? != *domain_model_struct_ident {
                return None;
            }

            // Check if the impl block functions have the right return type
            let functions = item_impl.items.iter().filter_map(|item| match item {
                syn::ImplItem::Fn(impl_item_fn) => {
                    let output_type = match &impl_item_fn.sig.output {
                        syn::ReturnType::Type(_, tipe) => get_type_path(tipe).ok()?,
                        _ => return None,
                    };

                    // Match Result<Vec<_>, processing_error>
                    let result_tipe = output_type.segments.last()?;
                    if result_tipe.ident != "Result" {
                        return None;
                    }

                    let generic_args = match &result_tipe.arguments {
                        syn::PathArguments::AngleBracketed(args) => args,
                        _ => return None,
                    };

                    let mut arg_pairs = generic_args.args.iter().filter_map(|arg| match arg {
                        syn::GenericArgument::Type(t) => Some(get_type_path(t).ok()?.segments),
                        _ => None,
                    });

                    // Match Vec<effect> and processing_error in Result
                    let left = arg_pairs.next()?;
                    let right = arg_pairs.next()?;
                    if right.last()?.ident == *processing_error && left.first()?.ident == "Vec" {
                        let inner_effect = match &left.last()?.arguments {
                            syn::PathArguments::AngleBracketed(args) => args.args.last()?,
                            _ => return None,
                        };
                        match inner_effect {
                            syn::GenericArgument::Type(t) => {
                                if get_type_path(t).ok()?.segments.last()?.ident == *effect {
                                    Some(impl_item_fn.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            });
            Some(functions.collect::<Vec<_>>())
        })
        .flatten()
        .collect::<Vec<ImplItemFn>>();
    if cqrs_fns.is_empty() {
        panic!(
            r#"Did not find a single cqrs-function! Be sure to implement them like:
            impl {domain_model_struct_ident}{{
                fn my_cqrs_function(app_state : & AppState, OPTIONALLY_ANY_OTHER_PARAMETERS) -> Result<Vec<{effect}, {processing_error}>>{{...}}
            }}
            "#
        )
    }
    cqrs_fns
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use quote::{format_ident, quote};
    use syn::{parse_str, Fields, FieldsNamed, File, ItemEnum, Token, Variant};

    use crate::utils::generate_cqrs_impl::{
        generate_cqrs_enum, generate_cqrs_functions, generate_cqrs_impl, get_cqrs_fns_sig,
        get_cqrs_fns_sig_idents, get_cqrs_fns_sig_tipes, get_cqrs_functions,
    };

    const CODE: &str = r#"
            impl NotMyModel {
                pub(crate) fn fake(
                    app_state: &AppState,
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
            impl MyGoodDomainModel {
                pub(crate) fn wrong_fn_effect_type(
                    app_state: &AppState,
                    todo_pos: usize,
                ) -> Result<Vec<MyWrongEffect>, MyGoodProcessingError> {
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
                pub(crate) fn wrong_fn_error_type(
                    app_state: &AppState,
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
                    app_state: &AppState,
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
            impl MyGoodDomainModel {
                pub fn get_items_as_string(&self) -> Vec<String> {
                    self.items.iter().map(|item| item.text.clone()).collect()
                }

                pub(crate) fn add_item(
                    app_state: &AppState,
                    item: String,
                    priority: usize,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    let model_lock = Self::get_model_lock(app_state);
                    model_lock
                        .blocking_write()
                        .items
                        .push(DomainItem { text: item });
                    app_state.mark_dirty();
                    // this clone is cheap, as it is on ARC (RustAutoOpaque>T> = Arc<RwMutex<T>>)
                    Ok(vec![MyGoodDomainModelEffect::RenderItems(
                        model_lock.clone(),
                    )])
                }
                pub(crate) fn clean_list(
                    app_state: &AppState,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    let model_lock = Self::get_model_lock(app_state);
                    model_lock.blocking_write().items.clear();
                    app_state.mark_dirty();
                    Ok(vec![MyGoodDomainModelEffect::RenderItems(
                        model_lock.clone(),
                    )])
                }
                pub(crate) fn get_all_items(
                    app_state: &AppState,
                ) -> Result<Vec<MyGoodDomainModelEffect>, MyGoodProcessingError> {
                    let model_lock = MyGoodDomainModel::get_model_lock(app_state);
                    Ok(vec![MyGoodDomainModelEffect::RenderItems(
                        model_lock.clone(),
                    )])
                }
            }
        "#;

    #[test]
    fn get_cqrs_fns_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let cqrs_functions = get_cqrs_functions(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        let result = quote! {
            #(#cqrs_functions)*
        };

        let expected = "pub (crate) fn remove_item (app_state : & AppState , todo_pos : usize ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; let items = & mut model_lock . blocking_write () . items ; if todo_pos > items . len () { Err (MyGoodProcessingError :: ItemDoesNotExist (todo_pos)) } else { items . remove (todo_pos - 1) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } } pub (crate) fn add_item (app_state : & AppState , item : String , priority : usize ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . push (DomainItem { text : item }) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn clean_list (app_state : & AppState ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . clear () ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn get_all_items (app_state : & AppState ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = MyGoodDomainModel :: get_model_lock (app_state) ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) }";
        assert_eq!(expected.to_string(), result.to_string());
    }
    #[test]
    #[should_panic = "Did not find a single cqrs-function! Be sure to implement them like:
            impl SomethingWrong{
                fn my_cqrs_function(app_state : & AppState, OPTIONALLY_ANY_OTHER_PARAMETERS) -> Result<Vec<MyGoodDomainModelEffect, MyGoodProcessingError>>{...}
            }"]
    fn get_cqrs_fns_fail_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let cqrs_functions = get_cqrs_functions(
            &format_ident!("SomethingWrong"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        let result = quote! {
            #(#cqrs_functions)*
        };

        let expected = "pub (crate) fn remove_item (app_state : & AppState , todo_pos : usize ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; let items = & mut model_lock . blocking_write () . items ; if todo_pos > items . len () { Err (MyGoodProcessingError :: ItemDoesNotExist (todo_pos)) } else { items . remove (todo_pos - 1) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } } pub (crate) fn add_item (app_state : & AppState , item : String ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . push (DomainItem { text : item }) ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn clean_list (app_state : & AppState ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = Self :: get_model_lock (app_state) ; model_lock . blocking_write () . items . clear () ; app_state . mark_dirty () ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) } pub (crate) fn get_all_items (app_state : & AppState ,) -> Result < Vec < MyGoodDomainModelEffect > , MyGoodProcessingError > { let model_lock = MyGoodDomainModel :: get_model_lock (app_state) ; Ok (vec ! [MyGoodDomainModelEffect :: RenderItems (model_lock . clone () ,)]) }";
        assert_eq!(expected.to_string(), result.to_string());
    }

    #[test]
    fn generate_cqrs_enum_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");

        // let result = generate_cqrs_enum(&get_cqrs_fns_sig_tipes(&get_cqrs_fns_sig(
        let result = generate_cqrs_enum(
            &get_cqrs_fns_sig_tipes(&get_cqrs_fns_sig(get_cqrs_functions(
                &format_ident!("MyGoodDomainModel"),
                &format_ident!("MyGoodDomainModelEffect"),
                &format_ident!("MyGoodProcessingError"),
                &ast,
            ))),
            &format_ident!("MyGoodDomainModel"),
        );
        let expected = quote! {
            pub enum Cqrs {
                MyGoodDomainModelRemoveItem(usize),
                MyGoodDomainModelAddItem(String, usize),
                MyGoodDomainModelCleanList,
                MyGoodDomainModelGetAllItems
            }
        };

        assert_eq!(expected.to_string(), result.to_string());
    }
    #[test]
    fn generate_cqrs_fns_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");

        let result = generate_cqrs_functions(
            &get_cqrs_fns_sig_idents(&get_cqrs_fns_sig(get_cqrs_functions(
                &format_ident!("MyGoodDomainModel"),
                &format_ident!("MyGoodDomainModelEffect"),
                &format_ident!("MyGoodProcessingError"),
                &ast,
            ))),
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &parse_str::<ItemEnum>(
                "
                       pub enum MyGoodDomainModelEffect {
                          RenderModel(String),
                          DeleteModel,
                          PostitionModel(String, Usize),
                       }
                   ",
            )
            .expect("Cannot parse test oracle!")
            .variants
            .into_pairs()
            .map(|punctuated| punctuated.value().to_owned())
            .collect::<Vec<Variant>>(),
            &format_ident!("MyGoodProcessingError"),
        );
        // TODO check if comma after arg type is invalid rust code
        let expected = quote! {
        impl Cqrs {
            pub(crate) fn process_with_app_state(
                self,
                app_state: &AppState,
            ) -> Result<Vec<Effect>, ProcessingError> {
                let result = match self {
                    Cqrs::MyGoodDomainModelRemoveItem(todo_pos) => MyGoodDomainModel::remove_item(app_state, todo_pos),
                    Cqrs::MyGoodDomainModelAddItem(item, priority) => MyGoodDomainModel::add_item(app_state, item, priority),
                    Cqrs::MyGoodDomainModelCleanList => MyGoodDomainModel::clean_list(app_state),
                    Cqrs::MyGoodDomainModelGetAllItems => MyGoodDomainModel::get_all_items(app_state),
                }
                .map_err(ProcessingError::MyGoodProcessingError)?
                .into_iter()
                .map(|effect| match effect {
                    MyGoodDomainModelEffect::RenderModel(content) => {
                        Effect::MyGoodDomainModelRenderModel(content)
                    }
                    MyGoodDomainModelEffect::DeleteModel => {
                        Effect::MyGoodDomainModelDeleteModel
                    }
                })
                .collect();
                Ok(result)
            }
            pub fn process(self) -> Result<Vec<Effect>, ProcessingError> {
                let app_state = &Lifecycle::get().app_state;
                let result = self.process_with_app_state(app_state)?;
                let _ = app_state.persist().map_err(ProcessingError::NotPersisted);
                Ok(result)
            }
        }
                };

        assert_eq!(expected.to_string(), result.to_string());
    }
}
