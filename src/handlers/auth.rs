use actix_identity::Identity;
use actix_web::{web, HttpResponse};
use chrono_tz::Tz;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors;
use crate::models::{self, Selectable};
use crate::utils;

#[derive(Deserialize)]
pub struct ReqBody {
    email: String,
    password: String,
    tz: Tz,
}

#[derive(Serialize)]
struct ResBody {
    name: String,
    tz: Tz,
    timescale: String,
    allocations: Vec<models::ResAllocation>,
}

pub async fn login(
    req: web::Json<ReqBody>,
    id: Identity,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let authed_user = web::block(move || {
        let conn = pool.get().unwrap();
        req.into_inner().to_authed(&conn)
    })
    .await?;

    let identity = serde_json::to_string(&authed_user).unwrap();
    id.remember(identity);
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_me(
    id: Identity,
    user: models::AuthedUser,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    match web::block(move || {
        let conn = pool.get().unwrap();
        user.to_res(&conn)
    })
    .await
    {
        Ok(res_body) => Ok(HttpResponse::Ok().json(&res_body)),
        Err(errors::BlockingError::Error(errors::DbError::NotFound)) => {
            id.forget();
            Err(errors::ServiceError::Unauthorized)
        }
        _ => Err(errors::ServiceError::InternalServerError),
    }
}

pub async fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().finish()
}

impl ReqBody {
    fn to_authed(&self, conn: &models::Conn) -> Result<models::AuthedUser, errors::ServiceError> {
        use crate::schema::users::dsl::{email, users};

        if let Ok(user) = users
            .filter(email.eq(&self.email))
            .first::<models::User>(conn)
        {
            if utils::verify(&user.hash, &self.password)? {
                return Ok(models::AuthedUser {
                    id: user.id,
                    tz: self.tz,
                });
            }
        }
        Err(errors::ServiceError::Unauthorized)
    }
}

impl models::AuthedUser {
    fn to_res(&self, conn: &models::Conn) -> Result<ResBody, errors::DbError> {
        use crate::schema::allocations::dsl::{allocations, owner};
        use crate::schema::users::dsl::users;

        // NotFound after DB reset despite having identity
        let user = users.find(self.id).first::<models::User>(conn)?;
        let _allocations = allocations
            .filter(owner.eq(&self.id))
            .select(models::Allocation::columns())
            .load::<models::Allocation>(conn)?
            .into_iter()
            .map(|alc| alc.into())
            .collect::<Vec<models::ResAllocation>>();

        Ok(ResBody {
            name: user.name,
            tz: self.tz,
            timescale: user.timescale,
            allocations: _allocations,
        })
    }
}
