use actix_web::{web, HttpResponse};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::errors;
use crate::models;

#[derive(Deserialize)]
pub struct ReqBody {
    tasks: Vec<i32>,
    token: Option<String>,
}

#[derive(Serialize)]
pub struct ResBody {
    token: Option<String>,
}

pub async fn delete(
    req: web::Json<ReqBody>,
    user: models::AuthedUser,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let res_body = web::block(move || {
        use crate::schema::tasks::dsl::{id, tasks};

        let conn = pool.get().unwrap();
        let req = req.into_inner();
        match req.token {
            None => {
                req.accept(&user, &conn)?;
                // create one-time token
                let token = user.get_token(&conn)?.id.to_string();
                Ok(ResBody {
                    token: Some(token),
                })
            },
            Some(s) => {
                user.verify_token(&*s, &conn)?;
                // perform deletion
                diesel::delete(
                    tasks.filter(id.eq_any(req.tasks))
                ).execute(&conn)?;
                Ok(ResBody {
                    token: None,
                })
            },
        }
    })
    .await?;

    Ok(match res_body.token {
        Some(_) => HttpResponse::Accepted().json(res_body),
        None => HttpResponse::Ok().json(res_body),
    })
}

impl ReqBody {
    fn accept(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<(), errors::ServiceError> {
        use crate::schema::tasks::dsl::{assign, id, tasks};

        if let Some(tid) = tasks
            .select(id)
            .filter(id.eq_any(&self.tasks))
            .filter(assign.ne(&user.id))
            .first::<i32>(conn).ok() {
                return Err(errors::ServiceError::BadRequest(format!(
                    "#{}: not your item.",
                    tid
                )));
        }
        Ok(())
    }
}
