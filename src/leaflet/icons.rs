use std::rc::Rc;

use leaflet::Icon;

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
struct IconOptions {
    iconUrl: String,
    iconSize: Vec<u32>,
}

fn load_icon(name: &str) -> Icon {
    let url = format!("/{name}.png");
    let size = vec![28, 32];
    let options = IconOptions {
        iconUrl: url,
        iconSize: size,
    };

    let options = serde_wasm_bindgen::to_value(&options).unwrap();
    let icon = Icon::new(&options);
    icon
}

macro_rules! icon_generator {
    (methods;) => {};

    (methods; $name:ident; $($tail:tt)*) => {
        pub fn $name<'a>(&'a mut self) -> Rc<Icon> {
            match self.$name {
                Some(ref v) => v.clone(),
                None => {
                    let icon = load_icon(stringify!($name));
                    self.$name = Some(Rc::new(icon));
                    return self.$name();
                }
            }
        }
        icon_generator!{methods; $($tail)*}
    };

}

#[derive(Default, Clone)]
pub struct IconGenerator {
    bump: Option<Rc<Icon>>,
    crack: Option<Rc<Icon>>,
    hole: Option<Rc<Icon>>,
    patch: Option<Rc<Icon>>,
}
impl IconGenerator {
    icon_generator! {methods; bump; crack; hole; patch;}
}
