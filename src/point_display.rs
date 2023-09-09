use std::{rc::Rc, str::FromStr};

use reqwest::{Method, Request};
use yew::{prelude::*, suspense::use_future};

#[derive(Properties)]
pub struct PointDisplayProps {
    pub leaflet: Rc<leaflet::Map>,
    pub clicked_point_info: Option<usize>,
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
    let info = match props.clicked_point_info {
        Some(v) => v,
        None => {
            return html!(<div class="pointview" style="flex: 0.0000001 0 auto; width:0px;">{"No point selected yet..."}</div>)
        }
    };

    let clear_clicked_cb = Callback::from({
        let cb = props.clear_clicked_cb.clone();
        move |e: MouseEvent| {
            e.prevent_default();
            cb.emit(());
        }
    });

    let fallback = html!(<SamplePointInfo sample_info={info.clone()} />);
    html! {
        <div class="pointview" style="flex: 4 0 auto; width:0px;">
            <div style="position: relative;">
                <Suspense {fallback}>
                    <DetailedPointInfo sample_info={info.clone()} />
                </Suspense>
                <button class="btn btn-outline-danger" onclick={clear_clicked_cb.clone()}>{"Unclick"}</button>
            </div>
            <button type="button" class="btn-close" style="position: absolute; top:0; right:0; z-index: 2; padding: 1.25rem 1rem;" onclick={clear_clicked_cb.clone()}></button>
        </div>
    }
}

#[derive(Properties, PartialEq, Debug)]
pub struct SampleInfo {
    pub sample_info: usize,
}

#[function_component]
pub fn SamplePointInfo(props: &SampleInfo) -> Html {
    let info = props.sample_info.clone();
    html!(
        <>
            <h1>{"Point "}{info}</h1>
            <p>{"Loading detailed info..."}</p>
        </>
    )
}

#[function_component]
pub fn DetailedPointInfo(props: &SampleInfo) -> HtmlResult {
    let response = use_future({
        move || {
            async move {
                // TODO: real HTTP request
                let client = reqwest::Client::new();
                let response = client
                    .execute(Request::new(
                        Method::GET,
                        reqwest::Url::from_str("https://httpbin.com/delay/2").unwrap(),
                    ))
                    .await;
                response
            }
        }
    })?;
    let info = props.sample_info.clone();
    Ok(html!(
        <>
            <h1>{"Point "}{info}</h1>

            <p>{"Detailed info loaded!"}</p>

        </>
    ))
}
