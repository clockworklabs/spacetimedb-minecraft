//! NBT serialization and deserialization for `Vec<Box<Entity>>` type.

use glam::IVec3;

use crate::entity::{self as e, 
    Entity, 
    Base, BaseKind, Projectile, ProjectileKind, Living, LivingKind,
    PaintingOrientation, PaintingArt};

use crate::serde::nbt::{NbtCompoundParse, NbtCompound, NbtParseError};
use crate::item::ItemStack;

use super::item_stack_nbt;
use super::slot_nbt;


pub fn from_nbt(comp: NbtCompoundParse) -> Result<Box<Entity>, NbtParseError> {

    let mut base = Base::default();
    base.persistent = true;

    // Position list.
    for (i, nbt) in comp.get_list("Pos")?.iter().enumerate().take(3) {
        base.pos[i] = nbt.as_double()?;
    }

    // Velocity list.
    for (i, nbt) in comp.get_list("Motion")?.iter().enumerate().take(3) {
        base.vel[i] = nbt.as_double()?;
    }

    // Yaw, pitch list.
    for (i, nbt) in comp.get_list("Rotation")?.iter().enumerate().take(2) {
        base.look[i] = nbt.as_float()?;
    }

    base.fall_distance = comp.get_float("FallDistance").unwrap_or_default();
    base.fire_time = comp.get_short("Fire").unwrap_or_default().max(0) as u32;
    base.air_time = comp.get_short("Air").unwrap_or_default().max(0) as u32;
    base.on_ground = comp.get_boolean("OnGround").unwrap_or_default();

    let id = comp.get_string("id")?;
    let base_kind = match id {
        "Item" => {

            base.lifetime = comp.get_int("Age").unwrap_or_default().max(0) as u32;
            
            let mut item = e::Item::default();
            item.health = comp.get_short("Health").unwrap_or_default().max(0) as u16;
            item.stack = item_stack_nbt::from_nbt(comp.get_compound("Item")?)?;
            BaseKind::Item(item)

        }
        "Painting" => BaseKind::Painting(e::Painting {
            block_pos: IVec3 {
                x: comp.get_int("TileX")?,
                y: comp.get_int("TileY")?,
                z: comp.get_int("TileZ")?,
            },
            orientation: match comp.get_byte("Dir")? {
                0 => PaintingOrientation::NegZ,
                1 => PaintingOrientation::NegX,
                2 => PaintingOrientation::PosZ,
                _ => PaintingOrientation::PosX,
            },
            art: PaintingArt::Kebab, // FIXME:
            ..Default::default()
        }),
        "PrimedTnt" => BaseKind::Tnt(e::Tnt {
            fuse_time: comp.get_byte("Fuse")?.max(0) as u32,
        }),
        "FallingSand" => BaseKind::FallingBlock(e::FallingBlock {
            block_id: comp.get_byte("Tile")? as u8,
            ..Default::default()
        }),
        "Minecart" => {
            BaseKind::Minecart(match comp.get_int("Type")? {
                1 => {
                    let mut inv: Box<[ItemStack; 27]> = Box::default();
                    slot_nbt::from_nbt_to_inv(comp.get_list("Items")?, &mut inv[..])?;
                    e::Minecart::Chest { inv }
                }
                2 => {
                    e::Minecart::Furnace { 
                        fuel: comp.get_short("fuel")?.max(0) as u32,
                        push_x: comp.get_double("PushX")?,
                        push_z: comp.get_double("PushZ")?,
                    }
                }
                _ => e::Minecart::Normal
            })
        }
        "Boat" => BaseKind::Boat(e::Boat::default()),
        "Arrow" |
        "Snowball" => {

            let mut projectile = Projectile::default();

            if comp.get_boolean("inGround")? {
                projectile.state = Some(e::ProjectileHit {
                    pos: IVec3 {
                        x: comp.get_short("xTile")? as i32,
                        y: comp.get_short("yTile")? as i32,
                        z: comp.get_short("zTile")? as i32,
                    }, 
                    block: comp.get_byte("inTile")? as u8, 
                    metadata: comp.get_byte("inData")? as u8,
                });
            }

            projectile.shake = comp.get_byte("shake")?.max(0) as u8;

            let projectile_kind = match id {
                "Arrow" => {
                    ProjectileKind::Arrow(e::Arrow {
                        from_player: comp.get_boolean("player").unwrap_or_default(),
                    })
                }
                "Snowball" => ProjectileKind::Snowball(e::Snowball::default()),
                _ => unreachable!()
            };

            BaseKind::Projectile(projectile, projectile_kind)

        }
        "Creeper" |
        "Skeleton" |
        "Spider" |
        "Giant" |
        "Zombie" |
        "Slime" |
        "Ghast" |
        "PigZombie" |
        "Pig" |
        "Sheep" |
        "Cow" |
        "Chicken" |
        "Squid" |
        "Wolf" => {

            
            let mut living = Living::default();
            living.health = comp.get_short("Health").unwrap_or(10).max(0) as u16;
            living.hurt_time = comp.get_short("HurtTime")?.max(0) as u16;
            living.death_time = comp.get_short("DeathTime")?.max(0) as u16;
            living.attack_time = comp.get_short("AttackTime")?.max(0) as u16;

            let living_kind = match id {
                "Creeper" => LivingKind::Creeper(e::Creeper {
                    powered: comp.get_boolean("powered")?,
                    ignited_time: None,
                }),
                "Skeleton" => LivingKind::Skeleton(e::Skeleton::default()),
                "Spider" => LivingKind::Spider(e::Spider::default()),
                "Giant" => LivingKind::Giant(e::Giant::default()),
                "Zombie" => LivingKind::Zombie(e::Zombie::default()),
                "Slime" => LivingKind::Slime(e::Slime {
                    size: comp.get_int("Size")?.clamp(0, 255) as u8,
                    ..Default::default()
                }),
                "Ghast" => LivingKind::Ghast(e::Ghast::default()),
                "PigZombie" => LivingKind::PigZombie(e::PigZombie {
                    anger: comp.get_short("Anger")? != 0,
                }),
                "Pig" => LivingKind::Pig(e::Pig {
                    saddle: comp.get_boolean("Saddle")?,
                }),
                "Sheep" => LivingKind::Sheep(e::Sheep {
                    sheared: comp.get_boolean("Sheared")?,
                    color: comp.get_byte("Color")? as u8,
                }),
                "Cow" => LivingKind::Cow(e::Cow::default()),
                "Chicken" => LivingKind::Chicken(e::Chicken::default()),
                "Squid" => LivingKind::Squid(e::Squid::default()),
                "Wolf" => LivingKind::Wolf(e::Wolf {
                    angry: comp.get_boolean("Angry")?,
                    sitting: comp.get_boolean("Sitting")?,
                    owner: {
                        let owner = comp.get_string("Owner")?;
                        (!owner.is_empty()).then(|| owner.to_string())
                    },
                }),
                _ => unreachable!()
            };

            BaseKind::Living(living, living_kind)

        }
        _ => return Err(NbtParseError::new(format!("{}/id", comp.path()), "valid entity id"))
    };

    let mut entity = Box::new(Entity(base, base_kind));
    entity.resize(); // Set the initial size/bounding box.

    Ok(entity)

}

pub fn to_nbt<'a>(comp: &'a mut NbtCompound, entity: &Entity) -> Option<&'a mut NbtCompound> {

    let Entity(base, base_kind) = entity;

    match base_kind {
        BaseKind::Item(item) => {

            comp.insert("id", "Item");
            comp.insert("Age", base.lifetime);
            comp.insert("Health", item.health.min(i16::MAX as _) as i16);

            let mut item_comp = NbtCompound::new();
            item_stack_nbt::to_nbt(&mut item_comp, item.stack);
            comp.insert("Item", item_comp);

        }
        BaseKind::Painting(painting) => {
            comp.insert("id", "Painting");
            comp.insert("TileX", painting.block_pos.x);
            comp.insert("TileY", painting.block_pos.y);
            comp.insert("TileZ", painting.block_pos.z);
            comp.insert("Dir", match painting.orientation {
                PaintingOrientation::NegZ => 0i8,
                PaintingOrientation::NegX => 1,
                PaintingOrientation::PosZ => 2,
                PaintingOrientation::PosX => 3,
            });
            comp.insert("Motive", "Kebab");
        }
        BaseKind::Boat(_) => {
            comp.insert("id", "Boat");
        }
        BaseKind::Minecart(e::Minecart::Normal) => {
            comp.insert("id", "Minecart");
            comp.insert("Type", 0i32);
        }
        BaseKind::Minecart(e::Minecart::Chest { inv }) => {
            comp.insert("id", "Minecart");
            comp.insert("Type", 1i32);
            comp.insert("Items", slot_nbt::to_nbt_from_inv(&inv[..]));
        }
        &BaseKind::Minecart(e::Minecart::Furnace { push_x, push_z, fuel }) => {
            comp.insert("id", "Minecart");
            comp.insert("Type", 2i32);
            comp.insert("fuel", fuel.min(i16::MAX as _) as i16);
            comp.insert("PushX", push_x);
            comp.insert("PushZ", push_z);
        }
        BaseKind::LightningBolt(_) => return None, // Not serializable
        BaseKind::FallingBlock(falling_block) => {
            comp.insert("id", "FallingSand");
            comp.insert("Tile", falling_block.block_id);
        }
        BaseKind::Tnt(tnt) => {
            comp.insert("id", "PrimedTnt");
            comp.insert("Fuse", tnt.fuse_time.min(i8::MAX as _) as i8);
        }
        BaseKind::Projectile(projectile, projectile_kind) => {

            match projectile_kind {
                ProjectileKind::Arrow(arrow) => {
                    comp.insert("id", "Arrow");
                    comp.insert("player", arrow.from_player);
                }
                ProjectileKind::Snowball(_) => comp.insert("id", "Snowball"),
                ProjectileKind::Egg(_) => return None, // Not serializable
                ProjectileKind::Fireball(_) => return None, // Not serializable
                ProjectileKind::Bobber(_) => return None, // Not serializable
            }

            let block = projectile.state.unwrap_or_default();
            comp.insert("xTile", block.pos.x as i16);
            comp.insert("yTile", block.pos.y as i16);
            comp.insert("zTile", block.pos.z as i16);
            comp.insert("inTile", block.block);
            comp.insert("inData", block.metadata);
            comp.insert("inGround", projectile.state.is_some());
            comp.insert("shake", projectile.shake.min(i8::MAX as _) as i8);

        }
        BaseKind::Living(living, living_kind) => {

            match living_kind {
                LivingKind::Ghast(_) => comp.insert("id", "Ghast"),
                LivingKind::Slime(slime) => {
                    comp.insert("id", "Slime");
                    comp.insert("Size", slime.size as u32);
                }
                LivingKind::Pig(pig) => {
                    comp.insert("id", "Pig");
                    comp.insert("Saddle", pig.saddle);
                }
                LivingKind::Chicken(_) => comp.insert("id", "Chicken"),
                LivingKind::Cow(_) => comp.insert("id", "Cow"),
                LivingKind::Sheep(sheep) => {
                    comp.insert("id", "Sheep");
                    comp.insert("Sheared", sheep.sheared);
                    comp.insert("Color", sheep.color);
                }
                LivingKind::Squid(_) => comp.insert("id", "Squid"),
                LivingKind::Wolf(wolf) => {
                    comp.insert("id", "Wolf");
                    comp.insert("Angry", wolf.angry);
                    comp.insert("Sitting", wolf.sitting);
                    comp.insert("Owner", wolf.owner.clone().unwrap_or_default());
                }
                LivingKind::Creeper(creeper) => {
                    comp.insert("id", "Creeper");
                    comp.insert("powered", creeper.powered);
                }
                LivingKind::Giant(_) => comp.insert("id", "Giant"),
                LivingKind::PigZombie(pig_zombie) => {
                    comp.insert("id", "PigZombie");
                    comp.insert("Anger", pig_zombie.anger as i16);
                }
                LivingKind::Skeleton(_) => comp.insert("id", "Skeleton"),
                LivingKind::Spider(_) => comp.insert("id", "Spider"),
                LivingKind::Zombie(_) => comp.insert("id", "Zombie"),
                // Other living entities cannot be serialized.
                _ => return None, // Not serializable
            }

            comp.insert("Health", living.health.min(i16::MAX as _) as i16);
            comp.insert("HurtTime", living.hurt_time.min(i16::MAX as _) as i16);
            comp.insert("DeathTime", living.death_time.min(i16::MAX as _) as i16);
            comp.insert("AttackTime", living.attack_time.min(i16::MAX as _) as i16);

        }
    }

    // Inserting here to we don't insert if the entity cannot be serialized.
    comp.insert("Pos", &base.pos.to_array()[..]);
    comp.insert("Motion", &base.vel.to_array()[..]);
    comp.insert("Rotation", &base.look.to_array()[..]);
    comp.insert("FallDistance", base.fall_distance);
    comp.insert("Fire", base.fire_time.min(i16::MAX as _) as i16);
    comp.insert("Air", base.air_time.min(i16::MAX as _) as i16);
    comp.insert("OnGround", base.on_ground);

    Some(comp)

}
