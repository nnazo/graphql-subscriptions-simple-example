use crate::db::Storage;
use crate::gql::{MutationType, User};
use async_graphql::{Context, Object, Result, ID};
use derive_new::new;

#[derive(Clone, new)]
pub struct Message {
    pub(crate) id: ID,
    pub(crate) user_id: ID,
    pub(crate) text: String,
}

#[Object]
/// A message sent by a user
impl Message {
    /// The ID of the message
    async fn id(&self) -> &str {
        &self.id
    }

    /// The text associated with the message
    async fn text(&self) -> &str {
        &self.text
    }

    /// The user that sent the message
    async fn user(&self, ctx: &Context<'_>) -> Result<User> {
        let db = ctx.data::<Storage>()?.lock().await;
        db.users
            .iter()
            .find(|(_, u)| u.id == self.user_id)
            .map_or_else(
                || Err(async_graphql::Error::new("User not found")),
                |(_, u)| Ok(u.clone()),
            )
    }
}

#[derive(Clone, new)]
pub struct MessageMutated {
    pub(crate) mutation_type: MutationType,
    pub(crate) id: ID,
}

#[Object]
/// An event for when a message is changed
impl MessageMutated {
    /// How the message was modified
    async fn mutation_type(&self) -> MutationType {
        self.mutation_type
    }

    /// The message that was mutated
    async fn message(&self, ctx: &Context<'_>) -> Result<Option<Message>> {
        let db = ctx.data::<Storage>()?.lock().await;
        let id = self.id.parse::<usize>()?;
        Ok(db.messages.get(id).cloned())
    }
}
