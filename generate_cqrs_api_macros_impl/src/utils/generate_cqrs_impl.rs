use std::iter;

use log::debug;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use stringcase::pascal_case_with_sep;
use syn::File;
use syn::Ident;
use syn::ImplItemFn;
use syn::Item;
use syn::ItemImpl;
use syn::Variant;

use crate::utils::get_enum::get_enum_by_ident_keyword;

use super::get_domain_model_struct::get_type_ident;
use super::get_domain_model_struct::get_type_path;

// TODO impl the fns here
// impl Cqrs {
//     pub(crate) fn process_with_app_state(
//         self,
//         app_state: &AppState,
//     ) -> Result<Vec<Effect>, ProcessingError> {
//         let result = match self {
//             Cqrs::TodoCommandAddTodo(todo) => TodoListModel::add_todo(app_state, todo),
//             Cqrs::TodoCommandRemoveTodo(todo_pos) => {
//                 TodoListModel::remove_todo(app_state, todo_pos)
//             }
//             Cqrs::TodoCommandCleanList => TodoListModel::clean_list(app_state),
//             Cqrs::TodoQueryAllTodos => TodoListModel::get_all_todos(app_state),
//         }
//         .map_err(ProcessingError::TodoListProcessingError)?
//         .into_iter()
//         .map(|effect| match effect {
//             TodoListEffect::RenderTodoList(content) => {
//                 Effect::TodoListEffectRenderTodoList(content)
//             }
//         })
//         .collect();
//         Ok(result)
//     }
//     pub fn process(self) -> Result<Vec<Effect>, ProcessingError> {
//         let app_state = &Lifecycle::get().app_state;
//         let result = self.process_with_app_state(app_state)?;
//         //persist the state, but only if dirty
//         let _ = app_state.persist().map_err(ProcessingError::NotPersisted);
//         Ok(result)
//     }
// }
pub(crate) fn generate_cqrs_impl(
    domain_struct_ident: &Ident,
    effect: &Ident,
    processing_error: &Ident,
    ast: &File,
) -> TokenStream {
    let generated_cqrs_enum =
        generate_cqrs_enum(domain_struct_ident, effect, processing_error, ast);

    quote! {
        #generated_cqrs_enum
    }
}

fn generate_cqrs_enum(
    domain_struct_ident: &Ident,
    effect: &Ident,
    processing_error: &Ident,
    ast: &File,
) -> TokenStream {
    let cqrs_functions = get_cqrs_functions(domain_struct_ident, effect, processing_error, ast);

    let cqrs_function_idents = cqrs_functions
        .iter()
        .map(|function| {
            let fn_ident = format_ident!(
                "{}{}",
                domain_struct_ident,
                pascal_case_with_sep(&function.sig.ident.to_string(), "_")
            );
            let arg_types = function
                .sig
                .inputs
                .iter()
                .filter_map(|arg| {
                    if let syn::FnArg::Typed(arg) = arg {
                        if let syn::Type::Path(path) = &*arg.ty {
                            path.path.get_ident()
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<&Ident>>();
            (fn_ident, arg_types)
        })
        .collect::<Vec<(Ident, Vec<&Ident>)>>();

    let enum_variants = cqrs_function_idents.iter().map(|(ident, args)| {
        if args.is_empty() {
            quote! {#ident}
        } else {
            quote! {
                #ident (#(#args,)*)
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

fn get_cqrs_functions(
    domain_struct_ident: &Ident,
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
            if get_type_ident(&item_impl.self_ty).ok()? != *domain_struct_ident {
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
            impl {domain_struct_ident}{{
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
    use syn::File;

    use crate::utils::generate_cqrs_impl::{
        generate_cqrs_enum, generate_cqrs_impl, get_cqrs_functions,
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
    fn generate_cqrs_impl_test() {
        let ast = syn::parse_file(CODE).expect("test oracle should be parsable");
        let result = generate_cqrs_enum(
            &format_ident!("MyGoodDomainModel"),
            &format_ident!("MyGoodDomainModelEffect"),
            &format_ident!("MyGoodProcessingError"),
            &ast,
        );
        // TODO check if comma after arg type is invalid rust code
        let expected = quote! {
            pub enum Cqrs {
                MyGoodDomainModelRemoveItem(usize, ),
                MyGoodDomainModelAddItem(String, usize, ),
                MyGoodDomainModelCleanList,
                MyGoodDomainModelGetAllItems
            }
        };

        assert_eq!(expected.to_string(), result.to_string());
    }
}
