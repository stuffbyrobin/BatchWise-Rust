//! Tenant members (`/members`): list the users of the caller's tenant and change
//! their role or active flag.
//!
//! Owners manage everyone; Managers manage Brewer, Sales and Viewer members
//! only (route authorisation keeps everyone else out). A tenant always keeps an
//! active Owner. Changes are written to the compliance audit log and apply on
//! the member's next request.

pub mod handler;
pub mod models;
pub mod repository;
pub mod service;

pub use handler::routes;
