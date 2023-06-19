use std::marker::PhantomData;

use axum::{
    body::Body, extract::State, handler::Handler, http::Request, routing::MethodRouter, Router,
};
use redfish_core::{
    auth::AuthenticateRequest,
    extract::RedfishAuth,
    privilege::{ConfigureComponents, Login},
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

pub struct DefaultPrivileges;
impl PrivilegeTemplate for DefaultPrivileges {
    type Get = Login;
    type Post = ConfigureComponents;
    type Put = ConfigureComponents;
    type Patch = ConfigureComponents;
    type Delete = ConfigureComponents;
    type Head = Login;
}

pub struct ComputerSystem<S, P>
where
    S: Clone,
{
    router: MethodRouter<S>,
    certificates: Option<Router<S>>,
    marker: PhantomData<fn() -> P>,
}

impl<S> Default for ComputerSystem<S, DefaultPrivileges>
where
    S: Clone,
{
    fn default() -> Self {
        Self {
            router: Default::default(),
            certificates: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<S, P> ComputerSystem<S, P>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
    P: PrivilegeTemplate + 'static,
    <P as PrivilegeTemplate>::Put: Send,
{
    pub fn put<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        self.router = self.router.put(
            |auth: RedfishAuth<P::Put>, State(state): State<S>, mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        );
        self
    }

    pub fn certificates(mut self, router: Router<S>) -> Self {
        self.certificates = Some(router);
        self
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router,
            certificates,
            ..
        } = self;
        Router::new()
            .route("/", router)
            .nest("/Certificates", certificates.unwrap())
    }
}
