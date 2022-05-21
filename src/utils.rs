pub fn is_sender_an_owner(from: &Option<&teloxide::types::User>, owner_id: u64) -> bool {
    if let Some(user) = from {
        user.id.0 == owner_id
    } else {
        false
    }
}
