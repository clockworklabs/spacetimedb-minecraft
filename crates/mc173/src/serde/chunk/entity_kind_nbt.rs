//! NBT serialization and deserialization for [`EntityKind`] enumeration.

use crate::entity::EntityKind;

pub fn from_nbt(id: &str) -> Option<EntityKind> {
    Some(match id {
        "Arrow" => EntityKind::Arrow,
        "Snowball" => EntityKind::Snowball,
        "Item" => EntityKind::Item,
        "Painting" => EntityKind::Painting,
        "Creeper" => EntityKind::Creeper,
        "Skeleton" => EntityKind::Skeleton,
        "Spider" => EntityKind::Spider,
        "Giant" => EntityKind::Giant,
        "Zombie" => EntityKind::Zombie,
        "Slime" => EntityKind::Slime,
        "Ghast" => EntityKind::Ghast,
        "PigZombie" => EntityKind::PigZombie,
        "Pig" => EntityKind::Pig,
        "Sheep" => EntityKind::Sheep,
        "Cow" => EntityKind::Cow,
        "Chicken" => EntityKind::Chicken,
        "Squid" => EntityKind::Squid,
        "Wolf" => EntityKind::Wolf,
        "PrimedTnt" => EntityKind::Tnt,
        "FallingSand" => EntityKind::FallingBlock,
        "Minecart" => EntityKind::Minecart,
        "Boat" => EntityKind::Boat,
        _ => return None
    })
}

pub fn to_nbt(kind: EntityKind) -> Option<String> {
    Some(match kind {
        EntityKind::Item => "Item",
        EntityKind::Painting => "Painting",
        EntityKind::Boat => "Boat",
        EntityKind::Minecart => "Minecart",
        EntityKind::FallingBlock => "FallingSand",
        EntityKind::Tnt => "PrimedTnt",
        EntityKind::Arrow => "Arrow",
        EntityKind::Snowball => "Snowball",
        EntityKind::Ghast => "Ghast",
        EntityKind::Slime => "Slime",
        EntityKind::Pig => "Pig",
        EntityKind::Chicken => "Chicken",
        EntityKind::Cow => "Cow",
        EntityKind::Sheep => "Sheep",
        EntityKind::Squid => "Squid",
        EntityKind::Wolf => "Wolf",
        EntityKind::Creeper => "Creeper",
        EntityKind::Giant => "Giant",
        EntityKind::PigZombie => "PigZombie",
        EntityKind::Skeleton => "Skeleton",
        EntityKind::Spider => "Spider",
        EntityKind::Zombie => "Zombie",
        _ => return None
    }.to_string())
}
