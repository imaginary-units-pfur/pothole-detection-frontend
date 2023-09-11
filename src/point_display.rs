use std::rc::Rc;

use common_data::{DamageType, RoadDamage};
use yew::{prelude::*, suspense::use_future_with_deps};
use yew_hooks::use_previous;

use crate::{leaflet::icons::ShowIcon, SERVER_ADDR};

#[derive(Properties)]
pub struct PointDisplayProps {
    pub leaflet: Rc<leaflet::Map>,
    pub clicked_point_info: Option<RoadDamage>,
    pub clear_clicked_cb: Callback<()>,
}

impl PartialEq for PointDisplayProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::<leaflet::Map>::as_ptr(&self.leaflet) == Rc::<leaflet::Map>::as_ptr(&other.leaflet)
            && self.clicked_point_info == other.clicked_point_info
    }
}

#[function_component]
pub fn PointDisplay(props: &PointDisplayProps) -> Html {
    let previous_point = use_previous(props.clicked_point_info.clone());

    let mut do_hide = false;
    let info = match props.clicked_point_info {
        Some(ref v) => v, // If we have a value, just render the pane as is.
        None => match *previous_point {
            Some(ref v) => {
                do_hide = true; // If we have just lost this value, then render the pane with a style hiding it.
                v
            }
            None => {
                // If the value has been unavailable for a while, render a hidden pane.
                return html!(<div class="pointview" style="flex: 0.0000001 0 auto; width:0px;">{"No point selected yet..."}</div>);
            }
        },
    };

    let clear_clicked_cb = Callback::from({
        let cb = props.clear_clicked_cb.clone();
        move |e: MouseEvent| {
            e.prevent_default();
            cb.emit(());
        }
    });

    let fallback = html!(<SamplePointInfo sample_info={info.clone()} />);
    let style = if do_hide {
        "flex: 0.0000001 0 auto; width:0px;"
    } else {
        "flex: 4 0 auto; width:0px;"
    };
    html! {
        <div class="pointview" style={style}>
            <div style="position: relative;">
                <Suspense {fallback}>
                    <DetailedPointInfo sample_info={info.clone()} />
                </Suspense>
            </div>
            if !do_hide {
                <button type="button" class="btn-close" style="position: absolute; top:0; right:0; z-index: 2; padding: 1.25rem 1rem;" onclick={clear_clicked_cb.clone()}></button>
            }
        </div>
    }
}

#[derive(Properties, PartialEq, Debug)]
pub struct SampleInfo {
    pub sample_info: RoadDamage,
}

#[function_component]
pub fn SamplePointInfo(props: &SampleInfo) -> Html {
    let info = props.sample_info.clone();
    html!(
        <>
            <h1>{"Point "}{info.id}<ShowIcon damage_type={info.damage_type} /></h1>
            <h2><span class="placeholder col-7"></span></h2>
            <p><span class="placeholder col-7"></span></p>
            <p><span class="placeholder col-7"></span></p>
            <div class="card">
                <div class="card-body">
                    <div class="spinner-border" role="status">
                    </div>
                </div>
            </div>
            <hr />

            <h3>{"Analysis details"}</h3>
            <p><span class="placeholder col-3"></span><span class="placeholder text-bg-success col-7"></span></p>
            <p><span class="placeholder col-5"></span><span class="placeholder text-bg-info col-7"></span></p>
        </>
    )
}

#[function_component]
pub fn DetailedPointInfo(props: &SampleInfo) -> HtmlResult {
    let info = props.sample_info.clone();
    let response = use_future_with_deps(
        {
            |info: Rc<RoadDamage>| async move {
                let response = frontend_requests::get_info_by_id(SERVER_ADDR, info.id).await;
                prokio::time::sleep(std::time::Duration::from_secs_f32(0.2f32)).await;
                response
            }
        },
        info,
    )?;
    let info = props.sample_info.clone();
    match *response {
        Ok(ref more_info) => {
            let kind = match info.damage_type {
                DamageType::Alligator_crack => "Alligator crack",
                DamageType::Rutting_bump_pothole_separation => "Rutting/bump/pothole/separation",
                DamageType::Linear_longitudinal_crack => "Linear longitudinal crack",
                DamageType::White_line_blur => "White line blur",
                DamageType::Linear_lateral_crack => "Linear lateral crack",
                DamageType::Cross_walk_blur => "Cross walk blur",
                DamageType::Utility_hole_maintenance_hatch => "Utility hole/maintenance hatch",
                DamageType::Repair => "Repair",
                _ => "Unknown",
            };

            let kind_more = &more_info.top_type;

            let score = more_info.top_certainty;

            use crate::SERVER_ADDR;

            Ok(html!(
                <>
                    <h1>{"Point "}{info.id}<ShowIcon damage_type={info.damage_type} /></h1>

                    <h2>{"Kind: "}{kind}</h2>
                    <p>{"Longitude: "}{info.longitude}</p>
                    <p>{"Latitude: "}{info.latitude}</p>
                    <div class="card">
                        <div class="card-body">
                            <img style="width: 100%" src={format!("{SERVER_ADDR}/image/of-point/{}", info.id)} />
                        </div>
                    </div>
                    <hr />

                    <h3>{"Analysis details"}</h3>
                    <p>{"Label: "}<span class="text-success">{kind_more}</span></p>
                    <p>{"Confidence: "}<span class="text-info">{format!("{:.5}%", score*100.0)}</span>

                        <div class="progress" role="progressbar">
                            <div class="progress-bar bg-info" style={format!("width: {}%", score*100.0)}>
                            </div>
                        </div>

                    </p>


                </>
            ))
        }
        Err(ref why) => Ok(html!(
            <>
            <h1>{"Point "}{info.id}<ShowIcon damage_type={info.damage_type} /></h1>

            <div class="alert alert-danger nuh-uh">
                {"Could not fetch additional info: "}
                <code>{why}</code>
            </div>

            </>

        )),
    }
}
