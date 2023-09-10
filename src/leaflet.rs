mod icons;
mod layers;

use std::{collections::HashMap, marker, rc::Rc};

use common_data::{DamageType, RoadDamage};
use gloo_utils::document;
use leaflet::{LatLng, Map, Marker};
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{HtmlElement, Node};
use yew::prelude::*;
use yew_hooks::{use_async, use_debounce, use_is_first_mount};

use crate::{
    leaflet::{
        icons::IconGenerator,
        layers::{get_default_layer, get_matrix_layer, get_transport_layer},
    },
    point_display::PointDisplay,
    SERVER_ADDR,
};

#[derive(PartialEq, Properties, Clone)]
pub struct Props {
    #[prop_or_default]
    pub class: Classes,
    #[prop_or_default]
    pub style: AttrValue,
}

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[derive(Debug)]
    pub type MyTileLayer;

    #[wasm_bindgen(constructor, js_namespace = L, js_class="TileLayer")]
    pub fn new(url_template: &str, options: &JsValue) -> MyTileLayer;

    #[wasm_bindgen(method, js_class = "TileLayer")]
    pub fn addTo(this: &MyTileLayer, map: &Map);

    #[wasm_bindgen(method, js_class = "TileLayer")]
    pub fn remove(this: &MyTileLayer);
}

#[function_component]
pub fn MapComponent(props: &Props) -> Html {
    let leaflet_box = use_state(|| Option::<Rc<Map>>::None);
    let container_box = use_state(|| None);
    let current_layer = use_state(|| None);
    enum LayerStyle {
        Default,
        TransportDark,
        Matrix,
    }
    let current_layer_style = use_state(|| LayerStyle::Default);

    let markers: UseStateHandle<HashMap<i64, (RoadDamage, Rc<Marker>, (f64, f64), bool)>> =
        use_state(HashMap::new);

    let clicked_point_info = use_state(|| Option::None);

    // For ensuring that the map container is always the correct size,
    // as well as for invalidating its initial size of zero (before the element is drawn to the screen)
    let _leaflet_invalidate_size = yew_hooks::use_interval(
        {
            let leaflet_box = leaflet_box.clone();
            move || {
                if let Some(leaflet) = leaflet_box.as_ref() {
                    leaflet.invalidateSize(true);
                }
            }
        },
        200,
    );

    let perform_bbox_fetch: yew_hooks::UseAsyncHandle<(), Rc<anyhow::Error>> = use_async({
        let leaflet = leaflet_box.clone();
        let old_markers = markers.clone();
        let clicked_point_info = clicked_point_info.clone();
        async move {
            if let Some(leaflet) = leaflet.as_ref() {
                log::info!("Starting recalculating markers for area!");
                let bounds = leaflet.getBounds();
                let new_markers = {
                    let response = {
                        let ne = bounds.getNorthEast();
                        let sw = bounds.getSouthWest();
                        let bounds = frontend_requests::AABB {
                            p1: (ne.lng(), ne.lat()),
                            p2: (sw.lng(), sw.lat()),
                        };

                        frontend_requests::get_points_in_rect(SERVER_ADDR, bounds).await?
                    };

                    log::info!("Starting building markers");
                    let mut new_markers = HashMap::new();

                    let mut gen = IconGenerator::default();
                    for damage in response {
                        if old_markers.contains_key(&damage.id) {
                            // Skip processing an existing entry
                            continue;
                        }
                        let pos = LatLng::new(damage.latitude, damage.longitude);
                        let pos_raw = (damage.latitude, damage.longitude);
                        let marker = Marker::new(&pos);

                        let icon = match damage.damage_type {
                            DamageType::Crack => gen.crack(),
                            DamageType::Patch => gen.patch(),
                            DamageType::Pothole => gen.hole(),
                            DamageType::Other => gen.bump(), // TODO: fix this
                            _ => gen.bump(),                 // TODO: and this
                        };

                        marker.setIcon(&icon);

                        let click_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> =
                            Closure::new({
                                let clicked_point_info = clicked_point_info.clone();
                                let damage = damage.clone();
                                move |_e| {
                                    clicked_point_info.set(Some(damage.clone()));
                                }
                            });

                        marker.on("click", &click_handler.into_js_value());

                        marker.addTo(&leaflet);
                        new_markers.insert(damage.id, (damage, Rc::new(marker), pos_raw, true));
                    }
                    new_markers
                };

                let mut old_markers_values = (*old_markers).clone();
                old_markers_values.extend(new_markers.into_iter());
                old_markers.set(old_markers_values);
                log::info!("Markers are built!");
            }

            anyhow::Result::Ok(())
        }
    });

    let perform_bbox_fetch_loading = perform_bbox_fetch.loading;
    let perform_bbox_fetch_error = perform_bbox_fetch.error.clone();
    let perform_bbox_fetch_reset = {
        let perform_bbox_fetch = perform_bbox_fetch.clone();
        move || perform_bbox_fetch.update(())
    };

    let perform_bbox_fetch = use_debounce(move || perform_bbox_fetch.run(), 200);

    let (leaflet, container) = if use_is_first_mount() {
        // Initialize the target HTML element
        let container = document().create_element("div").unwrap();
        let container: HtmlElement = container.dyn_into().unwrap();
        container.set_class_name("map");
        let leaflet_map = Map::new_with_element(&container, &JsValue::NULL);

        leaflet_map.setView(&LatLng::new(55.34, 37.78), 11.0);

        let layer = get_default_layer();
        layer.addTo(&leaflet_map);
        current_layer_style.set(LayerStyle::Default);
        current_layer.set(Some(layer));

        container_box.set(Some(container.clone()));
        let leaflet_map = Rc::new(leaflet_map);

        let move_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new({
            let leaflet = leaflet_map.clone();
            let markers = markers.clone();

            move |_e| {
                //e.originalEvent().prevent_default();
                let bounds = leaflet.getBounds();
                log::info!("Moving map: current bounds are {:?}", bounds);

                let mut markers_value = (*markers).clone();

                let mut to_add = vec![];
                let mut to_remove = vec![];
                for (idx, (_damage, _marker, pos, is_present)) in markers_value.iter() {
                    let is_visible = bounds.contains(&LatLng::new(pos.0, pos.1));
                    match (is_visible, is_present) {
                        (true, false) => {
                            // Is not on map, but should be
                            to_add.push(*idx);
                        }
                        (false, true) => {
                            // Is on map, but shouldn't be
                            to_remove.push(*idx);
                        }
                        _ => {}
                    }
                }

                // Apply changes
                for idx in to_add {
                    let (_damage, marker, _pos, is_present) = markers_value.get_mut(&idx).unwrap();
                    marker.addTo(&leaflet);
                    log::info!("Adding marker {idx} to map");
                    *is_present = true;
                }
                for idx in to_remove {
                    let (_damage, marker, _pos, is_present) = markers_value.get_mut(&idx).unwrap();

                    marker.remove();
                    log::info!("Removing marker {idx} from map");
                    *is_present = false;
                }

                markers.set(markers_value);
            }
        });

        leaflet_map.on("move", &move_handler.into_js_value());

        let move_finish_handler: Closure<dyn FnMut(leaflet::MouseEvent) -> ()> = Closure::new({
            let perform_bbox_fetch = perform_bbox_fetch.clone();
            move |_e| {
                log::info!("Map drag is complete, querying for new marker state");
                perform_bbox_fetch_reset();
                perform_bbox_fetch.run()
            }
        });

        leaflet_map.on("moveend", &move_finish_handler.into_js_value());

        leaflet_box.set(Some(leaflet_map));

        // let map = leaflet_box.as_ref().unwrap();
        // (map, container)

        // Turns out that setting a value into a UseStateHandle doesn't happen immediately.
        // So, for the first render, return nothing
        // Because we set the UseStateHandle, a rerender will happen immediately.
        return html!();
    } else {
        let map = leaflet_box.as_ref().unwrap();
        let container = (*container_box.as_ref().unwrap()).clone();
        (map, container)
    };

    let set_default_style_cb = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_default_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::Default);
        })
    };
    let set_transport_style_cb: Callback<MouseEvent> = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_transport_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::TransportDark);
        })
    };

    let set_matrix_style_cb: Callback<MouseEvent> = {
        let leaflet = leaflet.clone();
        let current_layer = current_layer.clone();
        let current_layer_style = current_layer_style.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let new_layer = get_matrix_layer();
            let old_layer = current_layer.as_ref().unwrap(); // current_layer is always set when replaced
            old_layer.remove();
            new_layer.addTo(&leaflet);
            current_layer.set(Some(new_layer));
            current_layer_style.set(LayerStyle::Matrix);
        })
    };

    let map_style_choice = html! {
        <div class="btn-group" role="group">
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::Default) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_default_style_cb}>
                {"Default"}
            </button>
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::TransportDark) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_transport_style_cb}>
                {"TransportDark"}
            </button>
            <button class={classes!("btn", if matches!(*current_layer_style, LayerStyle::Matrix) {"btn-primary"} else {"btn-outline-primary"})} onclick={set_matrix_style_cb}>
                {"Matrix"}
            </button>
        </div>
    };

    let clear_clicked_cb = Callback::from({
        let clicked_point_id = clicked_point_info.clone();
        move |_| {
            clicked_point_id.set(None);
        }
    });

    // To render the map, need to create VRef to the map's element
    let node: &Node = &container.clone().into();
    let loading = perform_bbox_fetch_loading;
    let error = perform_bbox_fetch_error.is_some();
    let error_box = if let Some(why) = perform_bbox_fetch_error {
        html!(
            <div class="toast border border-danger align-items-center show mx-3 my-3 nuh-uh" role="alert" aria-live="assertive" aria-atomic="true" style="position: absolute; top:0; right:0; z-index: 10000;">
                <div class="d-flex">
                    <div class="toast-body">
                        {"Cannot fetch marker positions: "}
                        <code>{why}</code>
                    </div>
                </div>
            </div>
        )
    } else {
        html!()
    };
    html! {
        <div class={classes!("map-container", props.class.clone())}>
            <div class="map-and-pointview">

                // Main map widget inside a Card component
                <div class={classes!("card", "map-card", "mapview", loading.then_some("placeholder-wave"))}>
                    <div class={classes!("card-body", "map-card-body", loading.then_some("text-warning placeholder"), error.then_some("bg-danger"))} style={&props.style}>
                        {Html::VRef(node.clone())}
                        {error_box}
                    </div>
                    {map_style_choice}
                </div>

                // Side element containing info about clicked points

                <PointDisplay leaflet={leaflet.clone()} clicked_point_info={(*clicked_point_info).clone()} {clear_clicked_cb} />
            </div>
        </div>
    }
}
