mod app;
mod controllers;
mod services;
mod util;

pub mod dto;

pub use app::app;
pub use util::config::Config;
pub use util::cookies;
pub use util::error::ServiceResult;
pub use util::helpers::get_user_email;
pub use util::middleware;
pub use util::state;

pub use services::auth;

#[cfg(test)]
mod tests {}
