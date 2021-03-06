use actix_web::{web, HttpResponse};
use diesel::prelude::*;

use crate::errors;
use crate::models;

pub async fn star(
    tid: web::Path<i32>,
    user: models::AuthedUser,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let _ = web::block(move || {
        use crate::schema::permissions::dsl::*;
        use crate::schema::tasks::dsl::{is_starred, tasks};
        use diesel::dsl::{exists, select};

        let conn = pool.get().unwrap();
        let tid = tid.into_inner();
        let task = tasks.find(&tid).first::<models::Task>(&conn)?;
        if select(exists(
            permissions
                .filter(subject.eq(&user.id))
                .filter(object.eq(&task.assign))
                .filter(edit),
        ))
        .get_result(&conn)?
        {
            diesel::update(&models::Tid::from(tid))
                .set(is_starred.eq(&!task.is_starred))
                .execute(&conn)?;
            return Ok(());
        }
        Err(errors::ServiceError::BadRequest(
            "no edit permission.".into(),
        ))
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
