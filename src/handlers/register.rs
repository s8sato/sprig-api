use actix_web::{web, HttpResponse};
use chrono::NaiveTime;
use diesel::prelude::*;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use crate::errors;
use crate::models;
use crate::schema::users;
use crate::utils;

#[derive(Deserialize)]
pub struct ReqBody {
    key: uuid::Uuid,
    email: String,
    password: String,
    reset_pw: bool,
}

pub async fn register(
    req: web::Json<ReqBody>,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let _ = web::block(move || {
        let conn = pool.get().unwrap();
        let req = req.into_inner();
        if req.reset_pw {
            req.to_alt(&conn)?.update(&req, &conn)?;
        } else {
            req.to_new(&conn)?.insert(&conn)?;
        };
        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

impl ReqBody {
    fn to_new(&self, conn: &models::Conn) -> Result<NewUser, errors::ServiceError> {
        self.accept(conn)?;
        Ok(NewUser {
            email: self.email.to_owned(),
            hash: utils::hash(&self.password)?,
            name: self.email.to_owned(),
        })
    }
    fn to_alt(&self, conn: &models::Conn) -> Result<AltUser, errors::ServiceError> {
        self.accept(conn)?;
        Ok(AltUser {
            hash: Some(utils::hash(&self.password)?),
        })
    }
    fn accept(&self, conn: &models::Conn) -> Result<(), errors::ServiceError> {
        use crate::schema::invitations::dsl::{email, expires_at, invitations};

        diesel::delete(
            invitations.filter(expires_at.lt(&chrono::Utc::now()))
        ).execute(conn)?;
        if let Ok(invitation) = invitations
            .find(&self.key)
            .filter(email.eq(&self.email))
            .first::<models::Invitation>(conn) {
                diesel::delete(&invitation).execute(conn)?;
                return Ok(());
            }
        Err(errors::ServiceError::BadRequest(
            "invitation invalid.".into()
        ))
    }
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser {
    email: String,
    hash: String,
    name: String,
}

impl NewUser {
    fn insert(&self, conn: &models::Conn) -> Result<(), errors::ServiceError> {
        use crate::schema::allocations::dsl::allocations;
        use crate::schema::permissions::dsl::permissions;
        use crate::schema::users::dsl::users;

        let id = diesel::insert_into(users)
            .values(self)
            .get_result::<models::User>(conn)?
            .id;
        let permission = models::Permission {
            subject: id,
            object: id,
            edit: true,
        };
        diesel::insert_into(permissions)
            .values(&permission)
            .execute(conn)?;
        let allocation = models::Allocation {
            owner: id,
            open: NaiveTime::from_hms(9, 0, 0),
            hours: 6,
        };
        diesel::insert_into(allocations)
            .values(&allocation)
            .execute(conn)?;
        Ok(())
    }
}

#[derive(AsChangeset)]
#[table_name = "users"]
struct AltUser {
    hash: Option<String>,
}

impl AltUser {
    fn update(&self, req: &ReqBody, conn: &models::Conn) -> Result<(), errors::ServiceError> {
        use crate::schema::users::dsl::{email, users};

        let old_user = users
            .filter(email.eq(&req.email))
            .first::<models::User>(conn)?;
        diesel::update(&old_user).set(self).execute(conn)?;
        Ok(())
    }
}

// instant credentials for demo use
#[derive(Serialize)]
pub struct ResBody {
    email: String,
    password: String,
}

pub async fn get_account(
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let res_body = web::block(move || {
        let conn = pool.get().unwrap();
        let email = loop {
            let rand: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();
            if !super::invite::user_exists(&rand, &conn)? {
                break rand;
            }
        };
        let user = NewUser {
            email: email.clone(),
            hash: utils::hash(&email)?,
            name: email,
        };
        user.insert(&conn)?;
        Ok(ResBody::from(user))
    })
    .await?;

    Ok(HttpResponse::Ok().json(&res_body))
}

impl From<NewUser> for ResBody {
    fn from(user: NewUser) -> Self {
        Self {
            email: user.email.clone(),
            password: user.email,
        }
    }
}
