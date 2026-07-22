use super::{
    backend::{SeedVcLocalBackend, VoiceModelBackend},
    state::BackendDescriptor,
};

pub fn list_backends() -> Vec<BackendDescriptor> {
    let backend = seed_vc();
    vec![BackendDescriptor {
        backend_id: backend.backend_id().to_owned(),
        display_name: backend.display_name().to_owned(),
        optional: true,
    }]
}

pub fn seed_vc() -> SeedVcLocalBackend {
    SeedVcLocalBackend
}
