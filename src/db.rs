use crate::gql::{Message, User};
use futures::lock::Mutex;
use slab::Slab;
use std::sync::Arc;

// Simple in-memory "database" for the sake of this example

#[derive(Default)]
pub struct Db {
    pub(crate) users: Slab<User>,
    pub(crate) messages: Slab<Message>,
}

pub type Storage = Arc<Mutex<Db>>;
