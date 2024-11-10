use syn::{Item, ItemUse};

pub(crate) fn get_use_statements(ast: &syn::File) -> Vec<&ItemUse> {
    ast.items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => Some(use_item),
            _ => None,
        })
        .collect::<Vec<&ItemUse>>()
}
