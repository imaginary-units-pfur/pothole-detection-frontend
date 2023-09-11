use std::rc::Rc;

use common_data::DamageType;
use leaflet::Icon;
use yew::prelude::*;

#[derive(serde::Serialize)]
#[allow(non_snake_case)]
struct IconOptions {
    iconUrl: String,
    iconSize: Vec<u32>,
    iconAnchor: Vec<u32>,
}

#[derive(Properties, PartialEq)]
pub struct ShowIconProps {
    pub damage_type: DamageType,
}

#[function_component]
pub fn ShowIcon(props: &ShowIconProps) -> Html {
    let icon_url = match props.damage_type {
        DamageType::Alligator_crack
        | DamageType::Linear_longitudinal_crack
        | DamageType::Linear_lateral_crack => "/crack.svg",
        DamageType::White_line_blur | DamageType::Cross_walk_blur => "/patch.svg",
        DamageType::Rutting_bump_pothole_separation
        | DamageType::Utility_hole_maintenance_hatch => "/hole.svg",
        DamageType::Repair => "/other.svg",
        _ => "/other.svg",
    };

    html!(
        <img src={icon_url} style="height: 1.5em; position: absolute; top:0; right:0; display:inline;"/>
    )
}

fn load_icon(name: &str) -> Icon {
    let url = format!("/{name}.png");
    let size = vec![28, 32];
    let anchor = vec![14, 32];
    let options = IconOptions {
        iconUrl: url,
        iconSize: size,
        iconAnchor: anchor,
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
    other: Option<Rc<Icon>>,
}
impl IconGenerator {
    icon_generator! {methods; bump; crack; hole; patch; other;}
}
