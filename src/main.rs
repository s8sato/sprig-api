#[macro_use]
extern crate diesel;

use actix_cors::Cors;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{cookie::SameSite, middleware, web, App, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

mod errors;
mod handlers;
mod models;
mod schema;
mod utils;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    std::env::set_var(
        "RUST_LOG",
        format!(
            "{}=DEBUG,actix_web=INFO,actix_server=INFO",
            utils::env_var("APP_NAME")
        ),
    );
    env_logger::init();

    let pool: models::Pool = r2d2::Pool::builder()
        .build(ConnectionManager::<PgConnection>::new(utils::env_var(
            "DATABASE_URL",
        )))
        .expect("Failed to create pool.");

    HttpServer::new(move || {
        let is_dev = utils::env_var("MODE") == "dev";
        App::new()
            .data(pool.clone())
            .wrap(middleware::Logger::default())
            .wrap(if is_dev {
                Cors::permissive()
            } else {
                Cors::default()
                    .allow_any_header()
                    .allow_any_method()
                    .allowed_origin(&utils::env_var("ACCESS_CONTROL_ALLOW_ORIGIN"))
                    .supports_credentials()
            })
            .wrap(IdentityService::new({
                let cookie = CookieIdentityPolicy::new(utils::SECRET_KEY.as_bytes())
                    .name("auth")
                    .path("/")
                    .max_age(86400)
                    .http_only(true);
                if is_dev {
                    cookie
                } else {
                    cookie.same_site(SameSite::None).secure(true)
                }
            }))
            .data(web::JsonConfig::default().limit(4096))
            .service(
                web::scope("/api")
                    .service(
                        web::resource("/invite").route(web::post().to(handlers::invite::invite)),
                    )
                    .service(
                        web::resource("/register")
                            .route(web::get().to(handlers::register::get_account))
                            .route(web::post().to(handlers::register::register)),
                    )
                    .service(
                        web::resource("/auth")
                            .route(web::get().to(handlers::auth::get_me))
                            .route(web::post().to(handlers::auth::login))
                            .route(web::delete().to(handlers::auth::logout)),
                    )
                    .service(
                        web::scope("/app")
                            .wrap_fn(|req, srv| {
                                use actix_identity::RequestIdentity;
                                if req.get_identity().is_some() {
                                    use actix_service::Service;
                                    srv.call(req)
                                } else {
                                    use actix_web::{dev, HttpResponse};
                                    use futures::future::{ok, Either};
                                    Either::Right(ok(dev::ServiceResponse::new(
                                        req.into_parts().0,
                                        HttpResponse::Unauthorized().finish(),
                                    )))
                                }
                            })
                            .configure(auth_protected),
                    ),
            )
    })
    .bind(format!("0.0.0.0:{}", utils::env_var("PORT")))?
    .run()
    .await
}

fn auth_protected(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/tasks")
            .route(web::get().to(handlers::app::home::home))
            .route(web::post().to(handlers::app::text::text))
            .route(web::put().to(handlers::app::exec::exec)),
    )
    .service(
        web::resource("/task/{tid}")
            .route(web::get().to(handlers::app::focus::focus))
            .route(web::put().to(handlers::app::star::star)),
    );
}
