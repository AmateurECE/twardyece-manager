use axum::{
    body::Body, extract::State, handler::Handler, http::Request, routing::MethodRouter, Router,
};
use redfish_core::{
    auth::AuthenticateRequest, extract::RedfishAuth, privilege::ConfigureComponents,
};

#[derive(Default)]
pub struct Certificate<S>(MethodRouter<S>)
where
    S: Clone;

impl<S> Certificate<S>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
{
    pub fn get<H, T>(self, handler: H) -> Self
    where
        H: Handler<T, S, Body>,
        T: 'static,
    {
        // The privilege "ConfigureManager" is the default required for the
        // Certificate component, but Redfish Privilege Mapping 1.3.1 specifies
        // a subordinate override for the component ComputerSystem.
        Self(self.0.get(
            |auth: RedfishAuth<ConfigureComponents>,
             State(state): State<S>,
             mut request: Request<Body>| async {
                request.extensions_mut().insert(auth.user);
                handler.call(request, state).await
            },
        ))
    }

    pub fn into_router(self) -> Router<S> {
        Router::new().route("/", self.0)
    }
}
