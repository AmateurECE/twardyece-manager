use std::marker::PhantomData;

use axum::{
    body::Body, extract::State, handler::Handler, http::Request, routing::MethodRouter, Router,
};
use redfish_core::{
    auth::AuthenticateRequest,
    extract::RedfishAuth,
    privilege::{ConfigureManager, OperationPrivilegeMapping},
};

pub struct DefaultPrivileges;

impl OperationPrivilegeMapping for DefaultPrivileges {
    type Get = ConfigureManager;
    type Post = ConfigureManager;
    type Put = ConfigureManager;
    type Patch = ConfigureManager;
    type Delete = ConfigureManager;
    type Head = ConfigureManager;
}

pub struct Certificate<S, P>
where
    S: Clone,
{
    router: MethodRouter<S>,
    marker: PhantomData<fn() -> P>,
}

impl<S> Default for Certificate<S, DefaultPrivileges>
where
    S: Clone,
{
    fn default() -> Self {
        Self {
            router: Default::default(),
            marker: Default::default(),
        }
    }
}

impl<S, P> Certificate<S, P>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
    P: OperationPrivilegeMapping + 'static,
    <P as OperationPrivilegeMapping>::Get: Send,
{
    pub fn new() -> Self {
        Self {
            router: Default::default(),
            marker: Default::default(),
        }
    }

    pub fn get<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        self.router = self.router.get(
            |auth: RedfishAuth<P::Get>, State(state): State<S>, mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        );
        self
    }

    pub fn into_router(self) -> Router<S> {
        Router::new().route("/", self.router)
    }
}
