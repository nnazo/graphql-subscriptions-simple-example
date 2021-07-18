use crate::db::Storage;
use crate::gql::Message;
use async_graphql::{Context, Object, Result, ID};
use derive_new::new;

#[derive(Clone, new)]
pub struct User {
    pub(crate) id: ID,
    pub(crate) name: String,
}

#[Object]
/// A user of the application
impl User {
    /// The ID of the user
    async fn id(&self) -> &str {
        &self.id
    }

    /// The user's name
    async fn name(&self) -> &str {
        &self.name
    }

    /// The messages sent by the user
    async fn messages(&self, ctx: &Context<'_>) -> Result<Vec<Message>> {
        let db = ctx.data::<Storage>()?.lock().await;
        Ok(db
            .messages
            .iter()
            .filter(|(_, m)| m.user_id == self.id)
            .map(|(_, m)| m)
            .cloned()
            .collect())
    }
}
