use chrono_tz::Tz;
use sendgrid::{Mail, SGClient};
use sparkpost::transmission::{EmailAddress, Message, Recipient, Transmission};
use std::str::FromStr;

use crate::errors;
use crate::models;
use crate::utils;

enum API {
    SendGrid,
    SparkPost,
}

pub struct Email {
    sender: String,
    from: String,
    to: String,
    subject: String,
    body: String,
}

impl FromStr for API {
    type Err = errors::ServiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SendGrid" => Ok(API::SendGrid),
            "SparkPost" => Ok(API::SparkPost),
            etc => {
                println!("Invalid Email API: {}", etc);
                Err(errors::ServiceError::InternalServerError)
            }
        }
    }
}

impl Email {
    pub fn send(&self) -> Result<(), errors::ServiceError> {
        match utils::env_var("EMAIL_API").parse::<API>()? {
            API::SendGrid => {
                let mail = Mail::new()
                    .add_from_name(&*self.sender)
                    .add_from(&*self.from)
                    .add_to((&*self.to, &*self.to).into())
                    .add_subject(&*self.subject)
                    .add_html(&*self.body);

                match SGClient::new(utils::env_var("SENDGRID_API_KEY")).send(mail) {
                    Ok(res) => {
                        println!("SendGrid Response:\n{:#?}", res);
                        Ok(())
                    }
                    Err(err) => {
                        println!("SendGrid Error:\n{:#?}", err);
                        Err(errors::ServiceError::InternalServerError)
                    }
                }
            }
            API::SparkPost => {
                let mut mail = Message::new(EmailAddress::new(&*self.from, &*self.sender));
                mail.add_recipient(Recipient::from(&*self.to))
                    .subject(&*self.subject)
                    .html(&*self.body);

                match Transmission::new(utils::env_var("SPARKPOST_API_KEY")).send(&mail) {
                    Ok(res) => {
                        println!("SparkPost Response:\n{:#?}", res);
                        Ok(())
                    }
                    Err(err) => {
                        println!("SparkPost Error:\n{:#?}", err);
                        Err(errors::ServiceError::InternalServerError)
                    }
                }
            }
        }
    }
}

impl From<models::Invitation> for Email {
    fn from(invitation: models::Invitation) -> Self {
        let app = utils::env_var("APP_NAME");
        let subject = format!(
            "{} {}",
            if invitation.forgot_pw {
                "Reset Password of"
            } else {
                "Invitation to"
            },
            app,
        );
        let body = format!(
            "\
            Your {} key is: <br>
            <span style=\"font-size: x-large; font-weight: bold;\">{}</span> <br>
            The key expires on: <br>
            <span style=\"font-weight: bold;\">{} in {}</span> <br>
            ",
            if invitation.forgot_pw {
                "reset"
            } else {
                "register"
            },
            invitation.id,
            invitation
                .expires_at
                .with_timezone(&invitation.tz.parse::<Tz>().unwrap())
                .format("%Y/%m/%d %a %H:%M") // RFC 3339
                .to_string(),
            invitation.tz,
        );
        Self {
            sender: app,
            from: utils::env_var("SENDING_EMAIL_ADDRESS"),
            to: invitation.email,
            subject: subject,
            body: body,
        }
    }
}
