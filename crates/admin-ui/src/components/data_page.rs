use yew::prelude::*;

use crate::components::{DataTable, PageHeader, data_table::ColumnDef};
use crate::models::collection::Collection as CollectionModel;

#[derive(Clone, PartialEq)]
struct UserRecord {
    id: String,
    email: String,
    email_visibility: bool,
    verified: bool,
    username: String,
    name: String,
    created: String,
    updated: String,
}

#[derive(Properties, PartialEq)]
pub struct DataPageProps {
    pub selected_collection: Option<CollectionModel>,
}

fn badge(text: &str, true_style: bool) -> Html {
    if true_style {
        html! {
            <span class="bg-[#e6f4ea] text-[#1e7e34] px-2 py-0.5 rounded font-label-xs text-label-xs">
                { text }
            </span>
        }
    } else {
        html! {
            <span class="bg-surface-container-highest text-on-surface-variant px-2 py-0.5 rounded font-label-xs text-label-xs">
                { text }
            </span>
        }
    }
}

#[function_component(DataPage)]
pub fn data_page(props: &DataPageProps) -> Html {
    let records = use_state(|| {
        vec![
            UserRecord {
                id: "JJ2YRU30FBG8MqX".into(),
                email: "test3@example.com".into(),
                email_visibility: false,
                verified: false,
                username: "u_fy6TDdqL6JEG4xu".into(),
                name: "John Doe".into(),
                created: "2022-07-02 07:51:26".into(),
                updated: "2022-07-02 07:51:26".into(),
            },
            UserRecord {
                id: "qEooBTHoAGkWUKc".into(),
                email: "test2@example.com".into(),
                email_visibility: false,
                verified: true,
                username: "u_Gq7dcZkjM9v2Sgv".into(),
                name: "N/A".into(),
                created: "2022-07-02 07:51:05".into(),
                updated: "2022-10-29 22:23:33".into(),
            },
        ]
    });

    let drawer_open = use_state(|| false);
    let drawer_open_clone = drawer_open.clone();
    let on_row_click = Callback::from(move |_user: UserRecord| {
        drawer_open_clone.set(true);
    });

    let on_search = Callback::from(|_query: String| {});
    let on_create = Callback::from(|_| {});

    let columns = vec![
           ColumnDef::new("id", "id", |r: &UserRecord| html! {
               <span class="font-code-md text-code-md bg-surface-container-high/50 px-2 py-0.5 rounded border border-outline-variant/30 text-on-surface">
                   { &r.id }
               </span>
           }).icon("key").sortable(true),

           ColumnDef::new("email", "email", |r: &UserRecord| html! {
               <span class="font-body-sm text-body-sm">{ &r.email }</span>
           }).icon("mail").sortable(true),

           ColumnDef::new("emailVisibility", "emailVisibility", |r: &UserRecord| {
               badge(if r.email_visibility { "True" } else { "False" }, r.email_visibility)
           }).icon("visibility"),

           ColumnDef::new("verified", "verified", |r: &UserRecord| {
               badge(if r.verified { "True" } else { "False" }, r.verified)
           }).icon("verified"),

           ColumnDef::new("username", "username", |r: &UserRecord| html! {
               <span class="font-code-md text-code-md text-on-surface-variant">{ &r.username }</span>
           }).icon("face"),

           ColumnDef::new("name", "name", |r: &UserRecord| {
               if r.name == "N/A" {
                   html! { <span class="font-body-sm text-body-sm text-outline italic">{ "N/A" }</span> }
               } else {
                   html! { <span class="font-body-sm text-body-sm">{ &r.name }</span> }
               }
           }).icon("person_outline"),

           ColumnDef::new("created", "created", |r: &UserRecord| html! {
               <span class="font-label-xs text-label-xs text-on-surface-variant">{ &r.created }</span>
           }).icon("schedule").sortable(true),

           ColumnDef::new("updated", "updated", |r: &UserRecord| html! {
               <span class="font-label-xs text-label-xs text-on-surface-variant">{ &r.updated }</span>
           }).icon("update").sortable(true),
        ];
    html! {
        <main class="flex-1 flex flex-col overflow-hidden relative">
            {
                if let Some(col) = &props.selected_collection {
                    html! {
                        <>
                            <PageHeader
                                collection_name={col.name.clone()}
                                on_search={on_search}
                                on_create={on_create}
                            />
                            <div class="flex-1 overflow-auto px-6 pb-6 custom-scrollbar">
                                <DataTable<UserRecord>
                                    columns={columns}
                                    data={(*records).clone()}
                                    selectable={true}
                                    on_row_click={on_row_click}
                                />
                            </div>
                        </>
                    }
                } else {
                    html! {
                        <div class="flex flex-col items-center justify-center h-full text-on-surface-variant opacity-60">
                            <p class="text-lg">{"Select a collection from the sidebar to view details"}</p>
                        </div>
                    }
                }
            }
        </main>
    }
}
