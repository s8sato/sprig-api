use actix_web::{web, HttpResponse};
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use diesel::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::max;

use crate::errors;
use crate::models::{self, Selectable};
use crate::schema::{tasks, users};
use crate::utils;

#[derive(Deserialize)]
pub struct ReqBody {
    pub text: String,
}

#[derive(Serialize)]
enum ResBody {
    Cmd(ResCmd),
    Tasks { created: i32, updated: i32 },
}

pub async fn text(
    req: web::Json<ReqBody>,
    user: models::AuthedUser,
    pool: web::Data<models::Pool>,
) -> Result<HttpResponse, errors::ServiceError> {
    let req = req.into_inner().wash().parse::<Req>()?;

    let res_body = web::block(move || {
        let conn = pool.get().unwrap();
        match req {
            Req::Cmd(cmd) => Ok(ResBody::Cmd(match cmd {
                // TODO /alias
                ReqCmd::Help => ResCmd::Help(cmd_help("root.md")?),
                ReqCmd::User(req) => ResCmd::User(req.handle(&user, &conn)?),
                ReqCmd::Search(req) => ResCmd::Search(req.handle(&user, &conn)?),
                ReqCmd::Tutorial => ResCmd::Tutorial(cmd_help("tutorial.md")?),
                ReqCmd::Coffee => {
                    return Err(errors::ServiceError::BadRequest("I'm a teapot.".into()))
                }
            })),
            Req::Tasks(tasks) => Ok(tasks.read(&user)?.accept(&user, &conn)?.upsert(&conn)?),
        }
    })
    .await?;

    Ok(HttpResponse::Ok().json(res_body))
}

#[derive(Debug, PartialEq)]
pub enum Req {
    Cmd(ReqCmd),
    Tasks(ReqTasks),
}

#[derive(Debug, PartialEq)]
pub enum ReqCmd {
    Help,
    User(ReqUser),
    Search(ReqSearch),
    Tutorial,
    Coffee,
}

#[derive(Debug, PartialEq)]
pub enum ReqUser {
    Help,
    Info,
    Modify(ReqModify),
}

#[derive(Debug, PartialEq)]
pub enum ReqSearch {
    Help,
    Condition(Condition),
}

#[derive(Debug, PartialEq)]
pub enum ReqModify {
    Email(String),
    Password(PasswordSet),
    Name(String),
    Timescale(Timescale),
    Allocations(Vec<ReqAllocation>),
    Permission(ReqPermission),
}

#[derive(Debug, PartialEq)]
pub struct PasswordSet {
    pub old: String,
    pub new: String,
    pub confirmation: String,
}

pub type ReqAllocation = models::ResAllocation;

#[derive(Debug, PartialEq, Serialize)]
pub struct ReqPermission {
    pub user: String,
    pub permission: Option<bool>,
}

#[derive(Serialize)]
enum ResCmd {
    Help(String),
    User(ResUser),
    Search(ResSearch),
    Tutorial(String),
}

#[derive(Serialize)]
enum ResUser {
    Help(String),
    Info {
        email: String,
        since: DateTime<Utc>,
        executed: i32,
        tz: Tz,
        permissions: ResPermissions,
    },
    Modify(ResModify),
}

#[derive(Serialize)]
struct ResPermissions {
    view_to: Vec<String>,
    edit_to: Vec<String>,
    view_from: Vec<String>,
    edit_from: Vec<String>,
}

#[derive(Serialize)]
enum ResModify {
    Email(String),
    Password(()),
    Name(String),
    Timescale(String),
    Allocations(Vec<models::ResAllocation>),
    Permission(ResPermission),
}

type ResPermission = ReqPermission;

#[derive(Debug, PartialEq, Clone)]
pub enum Timescale {
    Year,
    Quarter,
    Month,
    Week,
    Day,
    Hours,
    Hour,
    Minutes,
    Minute,
    Second,
}

#[derive(Serialize)]
enum ResSearch {
    Help(String),
    Condition(Vec<models::ResTask>),
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct Condition {
    pub boolean: Boolean,
    pub context: Range<i32>, // TODO /s <#< not archived tasks only?
    pub weight: Range<f32>,
    pub startable: Range<models::EasyDateTime>,
    pub deadline: Range<models::EasyDateTime>,
    pub created_at: Range<models::EasyDateTime>,
    pub updated_at: Range<models::EasyDateTime>,
    pub title: Option<Expression>,
    pub assign: Option<Expression>,
    pub link: Option<Expression>,
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct Boolean {
    pub is_archived: Option<bool>,
    pub is_starred: Option<bool>,
    pub is_leaf: Option<bool>,
    pub is_root: Option<bool>,
}

type Range<T> = (Option<T>, Option<T>);

#[derive(Debug, PartialEq, PartialOrd)]
pub enum Expression {
    Words(Vec<String>),
    Regex(String),
}

#[derive(Debug, Default, PartialEq)]
pub struct ReqTasks {
    pub tasks: Vec<ReqTask>,
}

#[derive(Debug, Default, PartialEq)]
pub struct ReqTask {
    // indent #id joint] * TITLE startable- -deadline $weight @assign [joint link
    pub indent: i32,
    pub attribute: Attribute,
    pub link: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Attribute {
    pub is_starred: bool,
    pub id: Option<i32>,
    pub weight: Option<f32>,
    pub joint_head: Option<String>,
    pub joint_tails: Vec<String>,
    pub assign: Option<String>,
    pub startable: Option<models::EasyDateTime>,
    pub deadline: Option<models::EasyDateTime>,
    pub title: String,
}

#[derive(AsChangeset)]
#[table_name = "users"]
struct AltUser {
    email: Option<String>,
    hash: Option<String>,
    name: Option<String>,
    timescale: Option<String>,
}

impl ReqUser {
    fn handle(
        self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<ResUser, errors::ServiceError> {
        let res = match self {
            Self::Help => ResUser::Help(cmd_help("user.md")?),
            Self::Info => self.info(user, conn)?,
            Self::Modify(req) => ResUser::Modify(req.exec(user, conn)?),
        };
        Ok(res)
    }
    fn info(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<ResUser, errors::ServiceError> {
        use crate::schema::tasks::dsl::{assign, is_archived, tasks};
        use crate::schema::users::dsl::{created_at, email, users};

        let (email_, since) = users
            .find(user.id)
            .select((email, created_at))
            .first::<(String, DateTime<Utc>)>(conn)?;
        let executed = tasks
            .filter(assign.eq(&user.id))
            .filter(is_archived)
            .count()
            .get_result::<i64>(conn)? as i32;

        Ok(ResUser::Info {
            email: email_,
            since: since,
            executed: executed,
            tz: user.tz,
            permissions: user.permissions(conn)?,
        })
    }
}

impl models::AuthedUser {
    fn permissions(&self, conn: &models::Conn) -> Result<ResPermissions, errors::ServiceError> {
        Ok(ResPermissions {
            view_to: self.to(false, conn)?,
            edit_to: self.to(true, conn)?,
            view_from: self.from(false, conn)?,
            edit_from: self.from(true, conn)?,
        })
    }
    fn to(&self, edit_: bool, conn: &models::Conn) -> Result<Vec<String>, errors::DbError> {
        use crate::schema::permissions::dsl::*;
        use crate::schema::users::dsl::{id, name, users};
        use diesel::dsl::exists;
        users
            .select(name)
            .filter(exists(
                permissions
                    .filter(subject.eq(&self.id))
                    .filter(object.eq(id))
                    .filter(edit.eq(edit_)),
            ))
            .load::<String>(conn)
    }
    fn from(&self, edit_: bool, conn: &models::Conn) -> Result<Vec<String>, errors::DbError> {
        use crate::schema::permissions::dsl::*;
        use crate::schema::users::dsl::{id, name, users};
        use diesel::dsl::exists;
        users
            .select(name)
            .filter(exists(
                permissions
                    .filter(subject.eq(id))
                    .filter(object.eq(&self.id))
                    .filter(edit.eq(edit_)),
            ))
            .load::<String>(conn)
    }
}

impl ReqModify {
    fn exec(
        self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<ResModify, errors::ServiceError> {
        use crate::schema::allocations::dsl::{allocations, owner};
        use crate::schema::permissions::dsl::*;
        use crate::schema::users::dsl::{email, id, name, users};
        use diesel::dsl::{exists, select};

        if let Self::Allocations(req) = self {
            let mut ins = Vec::new();
            for alc in &req {
                ins.push(alc.verify(user)?);
            }
            diesel::delete(allocations.filter(owner.eq(&user.id))).execute(conn)?;
            diesel::insert_into(allocations)
                .values(&ins)
                .execute(conn)?;
            return Ok(ResModify::Allocations(req));
        }
        if let Self::Permission(req) = self {
            let subject_ = users
                .select(id)
                .filter(name.eq(&req.user))
                .first::<i32>(conn)
                .map_err(|_| {
                    errors::ServiceError::BadRequest(format!("{}: user not found.", req.user))
                })?;
            diesel::delete(
                permissions
                    .filter(subject.eq(&subject_))
                    .filter(object.eq(&user.id)),
            )
            .execute(conn)?;
            if let Some(edit_) = req.permission {
                diesel::insert_into(permissions)
                    .values(&models::Permission {
                        subject: subject_,
                        object: user.id,
                        edit: edit_,
                    })
                    .execute(conn)?;
            }
            return Ok(ResModify::Permission(req));
        }
        let mut alt_user = AltUser {
            email: None,
            hash: None,
            name: None,
            timescale: None,
        };
        let res = match self {
            Self::Email(s) => {
                if select(exists(users.filter(email.eq(&s)))).get_result(conn)? {
                    return Err(errors::ServiceError::BadRequest(format!(
                        "email already in use: {}",
                        s,
                    )));
                }
                alt_user.email = Some(s.clone());
                ResModify::Email(s)
            }
            Self::Password(password_set) => {
                let hash = password_set.verify(user, conn)?;
                alt_user.hash = Some(hash);
                ResModify::Password(())
            }
            Self::Name(s) => {
                if select(exists(users.filter(name.eq(&s)))).get_result(conn)? {
                    return Err(errors::ServiceError::BadRequest(format!(
                        "username already in use: {}",
                        s,
                    )));
                }
                alt_user.name = Some(s.clone());
                ResModify::Name(s)
            }
            Self::Timescale(timescale) => {
                alt_user.timescale = Some(timescale.as_str().into());
                ResModify::Timescale(timescale.as_str().into())
            }
            _ => unreachable!(),
        };
        diesel::update(user).set(&alt_user).execute(conn)?;

        Ok(res)
    }
}

impl PasswordSet {
    fn verify(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<String, errors::ServiceError> {
        use crate::schema::users::dsl::users;

        let min_password_len = 8;
        let old_hash = users.find(user.id).first::<models::User>(conn)?.hash;
        if utils::verify(&old_hash, &self.old)? {
            if min_password_len <= self.new.len() {
                if self.new == self.confirmation {
                    let new_hash = utils::hash(&self.new)?;
                    return Ok(new_hash);
                }
                return Err(errors::ServiceError::BadRequest(format!(
                    "new password mismatched with confirmation.",
                )));
            }
            return Err(errors::ServiceError::BadRequest(format!(
                "password should be at least {} length.",
                min_password_len,
            )));
        }
        return Err(errors::ServiceError::BadRequest(format!(
            "current password seems to be wrong.",
        )));
    }
}

impl Timescale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Year => "Y",
            Self::Quarter => "Q",
            Self::Month => "M",
            Self::Week => "W",
            Self::Day => "D",
            Self::Hours => "6h",
            Self::Hour => "h",
            Self::Minutes => "15m",
            Self::Minute => "m",
            Self::Second => "s",
        }
    }
}

impl ReqAllocation {
    fn verify(
        &self,
        user: &models::AuthedUser,
    ) -> Result<models::Allocation, errors::ServiceError> {
        if let Some(time) = NaiveTime::from_hms_opt(self.open_h as u32, self.open_m as u32, 0) {
            if (1..=24).contains(&self.hours) {
                return Ok(models::Allocation {
                    owner: user.id,
                    open: time,
                    hours: self.hours,
                });
            }
            return Err(errors::ServiceError::BadRequest(
                "please specify 1 to 24 hours.".into(),
            ));
        }
        Err(errors::ServiceError::BadRequest(
            "time notation invalid.".into(),
        ))
    }
}

impl ReqSearch {
    fn handle(
        self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<ResSearch, errors::ServiceError> {
        let res = match self {
            Self::Help => ResSearch::Help(cmd_help("search.md")?),
            Self::Condition(con) => ResSearch::Condition(con.extract(user, conn)?),
        };
        Ok(res)
    }
}

impl Condition {
    fn extract(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<Vec<models::ResTask>, errors::ServiceError> {
        use crate::schema::arrows::dsl::arrows;

        let mut res_tasks = self.query(user, conn)?;
        self.filter_regex(&mut res_tasks)?;
        if max(self.context.0, self.context.1).is_some() {
            // TODO /s <#< load all arrows?
            let _arrows: models::Arrows = arrows.load::<models::Arrow>(conn)?.into();
            self.filter_context(&mut res_tasks, &_arrows);
        }
        Ok(res_tasks)
    }
    fn query(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<Vec<models::ResTask>, errors::ServiceError> {
        use crate::schema::arrows::dsl::*;
        use crate::schema::permissions::dsl::*;
        use crate::schema::tasks::dsl::*;
        use crate::schema::users::dsl::{name, users};
        use diesel::dsl::exists;

        let mut query = tasks
            .filter(exists(
                permissions
                    .filter(subject.eq(&user.id))
                    .filter(object.eq(assign)),
            ))
            .inner_join(users)
            .select(models::SelTask::columns())
            .into_boxed();

        if let Some(b) = &self.boolean.is_archived {
            query = query.filter(is_archived.eq(b))
        }
        if let Some(b) = &self.boolean.is_starred {
            query = query.filter(is_starred.eq(b))
        }
        if let Some(b) = &self.boolean.is_leaf {
            query = query.filter(exists(arrows.filter(target.eq(id))).eq(!b))
        }
        if let Some(b) = &self.boolean.is_root {
            query = query.filter(exists(arrows.filter(source.eq(id))).eq(!b))
        }
        if let Some(w) = &self.weight.0 {
            query = query.filter(weight.ge(w))
        }
        if let Some(w) = &self.weight.1 {
            query = query.filter(weight.le(w))
        }
        if let Some(dt) = &self.startable.0 {
            query = query.filter(startable.ge(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.startable.1 {
            query = query.filter(startable.le(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.deadline.0 {
            query = query.filter(deadline.ge(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.deadline.1 {
            query = query.filter(deadline.le(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.created_at.0 {
            query = query.filter(created_at.ge(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.created_at.1 {
            query = query.filter(created_at.le(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.updated_at.0 {
            query = query.filter(updated_at.ge(user.globalize(&dt)?))
        }
        if let Some(dt) = &self.updated_at.1 {
            query = query.filter(updated_at.le(user.globalize(&dt)?))
        }
        if let Some(Expression::Words(words)) = &self.title {
            for w in words {
                query = query.filter(title.like(format!("%{}%", w)))
            }
        }
        if let Some(Expression::Words(words)) = &self.assign {
            for w in words {
                query = query.filter(name.like(format!("%{}%", w)))
            }
        }
        if let Some(Expression::Words(words)) = &self.link {
            for w in words {
                query = query.filter(link.like(format!("%{}%", w)))
            }
        }
        Ok(query
            .order((is_starred.desc(), updated_at.desc()))
            .limit(100) // TODO /s limit up to 100?
            .load::<models::SelTask>(conn)?
            .into_iter()
            .map(|t| t.to_res())
            .collect())
    }
    fn filter_regex(&self, tasks: &mut Vec<models::ResTask>) -> Result<(), errors::ServiceError> {
        if let Some(Expression::Regex(regex)) = &self.title {
            let regex = Regex::new(&regex)?;
            tasks.retain(|t| regex.is_match(&t.title))
        }
        if let Some(Expression::Regex(regex)) = &self.assign {
            let regex = Regex::new(&regex)?;
            tasks.retain(|t| regex.is_match(&t.assign))
        }
        if let Some(Expression::Regex(regex)) = &self.link {
            let regex = Regex::new(&regex)?;
            tasks.retain(|t| regex.is_match(&**t.link.as_ref().unwrap_or(&String::new())));
        }
        Ok(())
    }
    fn filter_context(&self, tasks: &mut Vec<models::ResTask>, arrows: &models::Arrows) {
        if let Some(id) = self.context.0 {
            let ids = models::Tid::from(id).nodes_to(models::LR::Root, arrows);
            tasks.retain(|t| ids.iter().any(|id| *id == t.id))
        }
        if let Some(id) = self.context.1 {
            let ids = models::Tid::from(id).nodes_to(models::LR::Leaf, arrows);
            tasks.retain(|t| ids.iter().any(|id| *id == t.id))
        }
    }
}

struct Acceptor {
    tasks: Vec<TmpTask>,
    arrows: TmpArrows,
}

type TmpArrows = models::Arrows;

struct TmpTask {
    id: Option<i32>,
    title: String,
    assign: Option<String>,
    is_starred: bool,
    startable: Option<DateTime<Utc>>,
    deadline: Option<DateTime<Utc>>,
    weight: Option<f32>,
    link: Option<String>,
}

impl ReqTasks {
    fn read(self, user: &models::AuthedUser) -> Result<Acceptor, errors::ServiceError> {
        let iter = self.tasks.iter().enumerate().rev();
        let mut tmp_arrows = Vec::new();
        for (src, t) in iter.clone() {
            // dependencies by indents
            if let Some((tgt, _)) = iter
                .clone()
                .filter(|(idx, _)| *idx < src)
                .find(|(_, t_)| t_.indent < t.indent)
            {
                tmp_arrows.push(models::Arrow {
                    source: src as i32,
                    target: tgt as i32,
                });
            }
            // dependencies by joints
            iter.clone()
                .filter(|(_, t_)| match &t.attribute.joint_head {
                    Some(head) => t_.attribute.joint_tails.iter().any(|tail| tail == head),
                    _ => false,
                })
                .for_each(|(tgt, _)| {
                    tmp_arrows.push(models::Arrow {
                        source: src as i32,
                        target: tgt as i32,
                    });
                });
        }
        let mut tmp_tasks = Vec::new();
        for t in self.tasks {
            let mut startable = None;
            if let Some(dt) = t.attribute.startable {
                startable = Some(user.globalize(&dt)?)
            }
            let mut deadline = None;
            if let Some(dt) = t.attribute.deadline {
                deadline = Some(user.globalize(&dt)?)
            }
            tmp_tasks.push(TmpTask {
                id: t.attribute.id,
                title: t.attribute.title,
                assign: t.attribute.assign,
                is_starred: t.attribute.is_starred,
                startable: startable,
                deadline: deadline,
                weight: t.attribute.weight,
                link: t.link,
            })
        }
        Ok(Acceptor {
            tasks: tmp_tasks,
            arrows: tmp_arrows.into(),
        })
    }
}

struct Upserter {
    tasks: Vec<TmpTaskOk>,
    arrows: TmpArrows,
}

struct TmpTaskOk {
    id: Option<i32>,
    title: String,
    assign: i32,
    is_starred: bool,
    startable: Option<DateTime<Utc>>,
    deadline: Option<DateTime<Utc>>,
    weight: Option<f32>,
    link: Option<String>,
}

impl Acceptor {
    fn accept(
        self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<Upserter, errors::ServiceError> {
        self.no_loop()?;
        self.valid_sd()?;
        self.valid_tid_use()?;
        self.valid_tid(user, conn)?;
        let assigns = self.valid_assign(user, conn)?;

        let tasks = self
            .tasks
            .into_iter()
            .zip(assigns.iter())
            .map(|(t, &a)| TmpTaskOk {
                id: t.id,
                title: t.title,
                assign: a,
                is_starred: t.is_starred,
                startable: t.startable,
                deadline: t.deadline,
                weight: t.weight,
                link: t.link,
            })
            .collect::<Vec<TmpTaskOk>>();

        Ok(Upserter {
            tasks: tasks,
            arrows: self.arrows,
        })
    }
    fn no_loop(&self) -> Result<(), errors::ServiceError> {
        if self.arrows.has_cycle() {
            return Err(errors::ServiceError::BadRequest("loop found.".into()));
        }
        Ok(())
    }
    fn valid_sd(&self) -> Result<(), errors::ServiceError> {
        if let Some(t) = self
            .tasks
            .iter()
            .filter(|t| t.deadline.is_some() && t.startable.is_some())
            .find(|t| t.deadline.unwrap() < t.startable.unwrap())
        {
            return Err(errors::ServiceError::BadRequest(format!(
                "{}... deadline then startable.",
                t.title.chars().take(8).collect::<String>(),
            )));
        }
        Ok(())
    }
    fn valid_tid_use(&self) -> Result<(), errors::ServiceError> {
        self.tid_unique()?;
        for path in self.arrows.paths() {
            self.tid_single_by(&path)?;
        }
        Ok(())
    }
    fn tid_unique(&self) -> Result<(), errors::ServiceError> {
        let mut ids = self.ids();
        ids.sort();
        let mut last = i32::MIN;
        for id in ids {
            if id == last {
                return Err(errors::ServiceError::BadRequest(format!(
                    "#{} appears multiple times.",
                    id,
                )));
            }
            last = id
        }
        Ok(())
    }
    fn ids(&self) -> Vec<i32> {
        self.tasks.iter().filter_map(|t| t.id).collect::<Vec<i32>>()
    }
    fn tid_single_by(&self, path: &models::Path) -> Result<(), errors::ServiceError> {
        let ids = path
            .iter()
            .filter_map(|idx| self.tasks.get(*idx as usize).unwrap().id)
            .collect::<Vec<i32>>();
        if 1 < ids.len() {
            return Err(errors::ServiceError::BadRequest(format!(
                "#{} -> #{} existing nodes wiring.",
                ids.get(0).unwrap(),
                ids.get(1).unwrap(),
            )));
        }
        Ok(())
    }
    fn valid_tid(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<(), errors::ServiceError> {
        use crate::schema::permissions::dsl::*;
        use crate::schema::tasks::dsl::{assign, tasks};
        use diesel::dsl::exists;

        for id in self.ids() {
            if tasks
                .find(id)
                .filter(exists(
                    permissions
                        .filter(subject.eq(&user.id))
                        .filter(object.eq(assign))
                        .filter(edit),
                ))
                .first::<models::Task>(conn)
                .is_err()
            {
                return Err(errors::ServiceError::BadRequest(format!(
                    "#{}: item not found, or no edit permission.",
                    id,
                )));
            }
        }
        Ok(())
    }
    fn valid_assign(
        &self,
        user: &models::AuthedUser,
        conn: &models::Conn,
    ) -> Result<Vec<i32>, errors::ServiceError> {
        use crate::schema::permissions::dsl::*;
        use crate::schema::users::dsl::{id, name, users};
        use diesel::dsl::exists;

        let mut assigns = Vec::new();
        for t in &self.tasks {
            let mut assign = user.id;
            if let Some(name_) = &t.assign {
                match users
                    .filter(name.eq(&name_))
                    .filter(exists(
                        permissions
                            .filter(subject.eq(&user.id))
                            .filter(object.eq(id))
                            .filter(edit),
                    ))
                    .first::<models::User>(conn)
                {
                    Ok(someone) => assign = someone.id,
                    Err(_) => {
                        return Err(errors::ServiceError::BadRequest(format!(
                            "@{}: user not found.",
                            name_,
                        )))
                    }
                }
            }
            assigns.push(assign)
        }
        Ok(assigns)
    }
}

#[derive(Insertable)]
#[table_name = "tasks"]
struct NewTask {
    title: String,
    assign: i32,
    is_starred: bool,
    startable: Option<DateTime<Utc>>,
    deadline: Option<DateTime<Utc>>,
    weight: Option<f32>,
    link: Option<String>,
}

#[derive(AsChangeset)]
#[table_name = "tasks"]
struct AltTask {
    title: Option<String>,
    assign: Option<i32>,
    is_starred: Option<bool>,
    startable: Option<Option<DateTime<Utc>>>,
    deadline: Option<Option<DateTime<Utc>>>,
    weight: Option<Option<f32>>,
    link: Option<Option<String>>,
}

impl Upserter {
    fn upsert(mut self, conn: &models::Conn) -> Result<ResBody, errors::ServiceError> {
        use crate::schema::arrows::dsl::arrows;
        use crate::schema::tasks::dsl::tasks;

        let mut permanents = Vec::new();
        let mut created = 0;
        let mut updated = 0;
        for t in self.tasks.into_iter() {
            let id = match t.id {
                None => {
                    let id = diesel::insert_into(tasks)
                        .values(&NewTask::from(t))
                        .get_result::<models::Task>(conn)?
                        .id;
                    created += 1;
                    id
                }
                Some(id) => {
                    diesel::update(tasks.find(id))
                        .set(&AltTask::from(t))
                        .execute(conn)?;
                    updated += 1;
                    id
                }
            };
            permanents.push(id)
        }
        for arw in &mut self.arrows.arrows {
            arw.source = *permanents.get(arw.source as usize).unwrap();
            arw.target = *permanents.get(arw.target as usize).unwrap();
        }
        diesel::insert_into(arrows)
            .values(&self.arrows.arrows)
            .execute(conn)?;

        Ok(ResBody::Tasks {
            created: created,
            updated: updated,
        })
    }
}

impl From<TmpTaskOk> for NewTask {
    fn from(tmp: TmpTaskOk) -> Self {
        Self {
            title: tmp.title,
            assign: tmp.assign,
            is_starred: tmp.is_starred,
            startable: tmp.startable,
            deadline: tmp.deadline,
            weight: tmp.weight,
            link: tmp.link,
        }
    }
}

impl From<TmpTaskOk> for AltTask {
    fn from(tmp: TmpTaskOk) -> Self {
        Self {
            title: Some(tmp.title),
            assign: Some(tmp.assign),
            is_starred: Some(tmp.is_starred),
            startable: Some(tmp.startable),
            deadline: Some(tmp.deadline),
            weight: Some(tmp.weight),
            link: Some(tmp.link),
        }
    }
}

fn cmd_help(filename: &str) -> std::io::Result<String> {
    let path = std::path::Path::new(&utils::env_var("CMD_HELP_DIR")).join(filename);
    std::fs::read_to_string(path)
}
