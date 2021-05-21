use actix_web::{web, HttpResponse};
use chrono_tz::Tz;
use diesel::prelude::*;
use serde::Deserialize;

use super::_email::Email;
use crate::errors;
use crate::models;

#[derive(Deserialize)]
pub struct ReqBody {
    email: String,
    forgot_pw: bool,
    tz: Tz,
}

pub async fn invite(
    req: web::Json<ReqBody>,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let _ = web::block(move || {
        let conn = pool.get().unwrap();
        let invitation: models::Invitation = req.into_inner().accept(&conn)?;
        dbg!(&invitation);
        Email::from(invitation).send()
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

impl ReqBody {
    fn accept(self, conn: &models::Conn) -> Result<models::Invitation, errors::ServiceError> {
        use crate::schema::invitations::dsl::invitations;

        let user_exists = user_exists(&self.email, conn)?;
        if user_exists && !self.forgot_pw {
            return Err(errors::ServiceError::BadRequest(
                "user already exists.".into(),
            ));
        }
        if !user_exists && self.forgot_pw {
            return Err(errors::ServiceError::BadRequest(
                "user does not exist yet.".into(),
            ));
        }
        let invitation: models::Invitation = self.into();

        Ok(diesel::insert_into(invitations)
            .values(&invitation)
            .get_result(conn)?)
    }
}

impl From<ReqBody> for models::Invitation {
    fn from(req: ReqBody) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            email: req.email,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
            forgot_pw: req.forgot_pw,
            tz: req.tz.to_string(),
        }
    }
}

pub fn user_exists(email_: &String, conn: &models::Conn) -> Result<bool, errors::ServiceError> {
    use crate::schema::users::dsl::{email, users};
    use diesel::dsl::{exists, select};

    let b: bool = select(exists(users.filter(email.eq(email_)))).get_result(conn)?;
    Ok(b)
}
