mod simple_broker;

mod message;
pub(crate) use message::*;
mod user;
pub(crate) use user::*;

use crate::db::Storage;
use async_graphql::{Context, Enum, Object, Result, Schema, Subscription, ID};
use futures::{Stream, StreamExt};
use simple_broker::SimpleBroker;
use std::time::Duration;

pub type ApiSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// All users
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let db = ctx.data::<Storage>()?.lock().await;
        Ok(db.users.iter().map(|(_, u)| u).cloned().collect())
    }

    /// All messages sent by any user
    async fn messages(&self, ctx: &Context<'_>) -> Result<Vec<Message>> {
        let db = ctx.data::<Storage>()?.lock().await;
        Ok(db.messages.iter().map(|(_, u)| u).cloned().collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create or update a user
    async fn save_user(&self, ctx: &Context<'_>, id: Option<ID>, name: String) -> Result<User> {
        let mut db = ctx.data::<Storage>()?.lock().await;
        match id {
            Some(id) => {
                let id = id.parse::<usize>().map_or_else(
                    |_e| Err(async_graphql::Error::new("ID is non-numeric")),
                    |id| Ok(id),
                )?;
                let user = db.users.get_mut(id).map_or_else(
                    || Err(async_graphql::Error::new("User not found")),
                    |u| Ok(u),
                )?;
                user.name = name;
                Ok(user.clone())
            }
            None => {
                let entry = db.users.vacant_entry();
                let id: ID = entry.key().into();
                let user = User::new(id, name);
                entry.insert(user.clone());
                Ok(user)
            }
        }
    }

    /// Create or update a message
    async fn save_message(
        &self,
        ctx: &Context<'_>,
        id: Option<ID>,
        user_id: Option<ID>,
        text: String,
    ) -> Result<Message> {
        let mut db = ctx.data::<Storage>()?.lock().await;
        let (message, mutation_type) = match id {
            Some(id) => {
                let id = id.parse::<usize>().map_or_else(
                    |_e| Err(async_graphql::Error::new("ID is non-numeric")),
                    |id| Ok(id),
                )?;
                let message = db.messages.get_mut(id).map_or_else(
                    || Err(async_graphql::Error::new("Message not found")),
                    |m| Ok(m),
                )?;
                message.text = text;
                (message.clone(), MutationType::Updated)
            }
            None => {
                let user_id = user_id.map_or_else(
                    || {
                        Err(async_graphql::Error::new(
                            "User ID not provided when ID not provided",
                        ))
                    },
                    |id| Ok(id),
                )?;
                let entry = db.messages.vacant_entry();
                let id: ID = entry.key().into();
                let message = Message::new(id, user_id, text);
                entry.insert(message.clone());
                (message, MutationType::Created)
            }
        };

        SimpleBroker::publish(MessageMutated::new(mutation_type, message.id.clone()));

        Ok(message)
    }

    /// Delete a message
    async fn delete_message(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let mut db = ctx.data::<Storage>()?.lock().await;
        let id = id.parse::<usize>()?;
        if db.messages.contains(id) {
            db.messages.remove(id);
            SimpleBroker::publish(message::MessageMutated::new(
                MutationType::Deleted,
                id.into(),
            ));
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[derive(Enum, Eq, PartialEq, Copy, Clone)]
pub enum MutationType {
    Created,
    Updated,
    Deleted,
}

pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// A stream of integers
    async fn interval(&self, #[graphql(default = 1)] n: i32) -> impl Stream<Item = i32> {
        let mut value = 0;
        async_stream::stream! {
            loop {
                futures_timer::Delay::new(Duration::from_secs(1)).await;
                value += n;
                yield value;
            }
        }
    }

    /// A stream of subscribed messages based on the query arguments
    async fn messages(
        &self,
        mutation_type: Option<MutationType>,
        id: Option<ID>,
    ) -> impl Stream<Item = MessageMutated> {
        SimpleBroker::<MessageMutated>::subscribe().filter(move |event| {
            let mutation_type_same = mutation_type.map_or_else(
                || true,
                |mutation_type| mutation_type == event.mutation_type,
            );
            let id_same = id.as_ref().map_or_else(|| true, |id| id == &event.id);
            async move { mutation_type_same && id_same }
        })
    }
}
