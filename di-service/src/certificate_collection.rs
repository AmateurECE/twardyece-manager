use std::marker::PhantomData;

use axum::{
    body::Body,
    extract::State,
    handler::Handler,
    http::{Request, StatusCode},
    routing::MethodRouter,
    Json, Router,
};
use redfish_codegen::registries::base::v1_15_0::Base;
use redfish_core::{
    auth::AuthenticateRequest, error, extract::RedfishAuth, privilege::ConfigureManager,
};

use super::OperationPrivilegeMapping;

pub struct DefaultPrivileges;
impl OperationPrivilegeMapping for DefaultPrivileges {
    type Get = ConfigureManager;
    type Post = ConfigureManager;
    type Put = ConfigureManager;
    type Patch = ConfigureManager;
    type Delete = ConfigureManager;
    type Head = ConfigureManager;
}

pub struct CertificateCollection<S, P>
where
    S: Clone,
{
    router: MethodRouter<S>,
    certificates: Option<Router<S>>,
    marker: PhantomData<fn() -> P>,
}

impl<S> Default for CertificateCollection<S, DefaultPrivileges>
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

impl<S, P> CertificateCollection<S, P>
where
    S: AsRef<dyn AuthenticateRequest> + Clone + Send + Sync + 'static,
    P: OperationPrivilegeMapping + 'static,
    <P as OperationPrivilegeMapping>::Get: Send,
{
    pub fn with_privileges() -> Self {
        Self {
            router: Default::default(),
            certificates: Default::default(),
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

    pub fn certificates(mut self, certificates: Router<S>) -> Self {
        self.certificates = Some(certificates);
        self
    }

    pub fn into_router(self) -> Router<S> {
        let Self {
            router,
            certificates,
            ..
        } = self;
        let result = Router::default();
        let result = match certificates {
            Some(certificates) => result.nest("/:certificate_id", certificates),
            None => result,
        };
        result.route(
            "/",
            router.fallback(|| async {
                (
                    StatusCode::METHOD_NOT_ALLOWED,
                    Json(error::one_message(Base::OperationNotAllowed.into())),
                )
            }),
        )
    }
}
