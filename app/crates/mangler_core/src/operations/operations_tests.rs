use super::{operation_list, default_image, OperationListItem};

#[test]
fn test_operation_list_not_empty() {
    let list = operation_list();
    assert!(!list.is_empty());
}

#[test]
fn test_default_image() {
    let img = default_image();
    assert_eq!(img.width(), 1);
    assert_eq!(img.height(), 1);
}

#[test]
fn test_all_operations_have_valid_settings() {
    fn check_items(items: &[OperationListItem]) {
        for item in items {
            match item {
                OperationListItem::Category { name, operation_list_items } => {
                    assert!(!name.is_empty());
                    check_items(operation_list_items);
                }
                OperationListItem::Operation { operation } => {
                    let settings = operation.settings();
                    assert!(!settings.name.is_empty());
                    let _inputs = operation.create_inputs();
                    let _outputs = operation.create_outputs();
                }
                OperationListItem::Subgraph => {}
            }
        }
    }
    check_items(&operation_list());
}
