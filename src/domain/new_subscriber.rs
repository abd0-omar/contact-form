use super::{subscriber_email::SubscriberEmail, subscriber_name::SubscriberName};

pub struct NewSubscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}
