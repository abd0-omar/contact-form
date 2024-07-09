// use super::{SubscriberEmail, SubscriberName};
use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;

#[derive(Debug)]
pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}
