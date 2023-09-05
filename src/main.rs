use yew::prelude::*;

fn main() {
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <main>
            <h1>{ "Hello World!" }</h1>
        </main>
    }
}
