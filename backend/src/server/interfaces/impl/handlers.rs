use crate::server::{
    config::AppState,
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    shared::handlers::{query::InterfaceQuery, traits::CrudHandlers},
};

impl CrudHandlers for Interface {
    type Service = InterfaceService;
    type FilterQuery = InterfaceQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.interface_service
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.created_at = existing.created_at
    }
}
