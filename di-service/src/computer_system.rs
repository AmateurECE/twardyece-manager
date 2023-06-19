use axum::{
    body::Body, extract::State, handler::Handler, http::Request, routing::MethodRouter, Router,
};
use redfish_core::{
    auth::AuthenticateRequest, extract::RedfishAuth, privilege::ConfigureComponents,
};

use super::PrivilegeTemplate;

pub struct CertificateCollectionPrivileges;
impl PrivilegeTemplate for CertificateCollectionPrivileges {
    type Get = ConfigureComponents;
    type Post = ConfigureComponents;
    type Put = ConfigureComponents;
    type Patch = ConfigureComponents;
    type Delete = ConfigureComponents;
    type Head = ConfigureComponents;
}

pub struct CertificatePrivileges;
impl PrivilegeTemplate for CertificatePrivileges {
    type Get = ConfigureComponents;
    type Post = ConfigureComponents;
    type Put = ConfigureComponents;
    type Patch = ConfigureComponents;
    type Delete = ConfigureComponents;
    type Head = ConfigureComponents;
}

#[derive(Default)]
pub struct ComputerSystem<S>
where
    S: Clone,
{
    router: MethodRouter<S>,
    certificates: Option<Router<S>>,
}

impl<S> ComputerSystem<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn put<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        let Self {
            router,
            certificates,
        } = self;
        Self {
            router: router.put(
                |auth: RedfishAuth<ConfigureComponents>,
                 State(state): State<S>,
                 mut request: Request<Body>| async {
                    request.extensions_mut().insert(auth.user);
                    handler.call(request, state).await
                },
            ),
            certificates,
        }
    }

    pub fn certificates(self, router: Router<S>) -> Self {
        Self {
            router: self.router,
            certificates: Some(router),
        }
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router,
            certificates,
        } = self;
        Router::new()
            .route("/", router)
            .nest("/Certificates", certificates.unwrap())
    }
}
