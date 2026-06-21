use yew_router::prelude::*;

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,

    #[at("/collection/:name")]
    Collection { name: String },
}
