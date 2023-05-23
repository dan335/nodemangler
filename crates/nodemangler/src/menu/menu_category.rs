use super::menu_button::MenuButton;

pub struct MenuCategory {
    title: String,
    children: Vec<MenuButton>,
}

impl MenuCategory {
    pub fn new(title: &str, children: Vec<MenuButton>) -> Self {
        Self {
            title: title.to_owned(),
            children,
        }
    }
}