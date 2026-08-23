use yew_router::prelude::*;

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[at("/")]
    Home,

    #[at("/login")]
    Login,

    #[at("/forgot-password")]
    ForgotPassword,

    #[at("/reset-password")]
    ResetPassword,

    #[at("/collection/:name")]
    Collection { name: String },

    #[at("/settings")]
    SettingsGeneral,

    #[at("/settings/general")]
    SettingsGeneralExplicit,

    #[at("/settings/mail")]
    SettingsMail,

    #[at("/settings/storage")]
    SettingsStorage,

    #[at("/settings/backups")]
    SettingsBackups,

    #[at("/settings/crons")]
    SettingsCrons,

    #[at("/logs")]
    Logs,

    #[not_found]
    #[at("/404")]
    NotFound,
}
