use argon2;
use once_cell::sync::Lazy;

use crate::errors;

pub static SECRET_KEY: Lazy<String> = Lazy::new(|| env_var("SECRET_KEY"));

pub fn env_var(x: &str) -> String {
    std::env::var(x).expect(format!("{} must be set", x).as_str())
}

pub fn hash(password: &str) -> Result<String, errors::ServiceError> {
    argon2::hash_encoded(
        password.as_bytes(),
        SECRET_KEY.as_bytes(),
        &argon2::Config::default(),
    )
    .map_err(|err| {
        dbg!(err);
        errors::ServiceError::InternalServerError
    })
}

pub fn verify(hash: &str, password: &str) -> Result<bool, errors::ServiceError> {
    argon2::verify_encoded(hash, password.as_bytes()).map_err(|err| {
        dbg!(err);
        errors::ServiceError::InternalServerError
    })
}
