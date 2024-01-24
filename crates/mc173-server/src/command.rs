//! Module for command handlers.

use std::mem;

use glam::IVec3;

use mc173::item::{self, ItemStack};
use mc173::world::{World, Event, Weather};
use mc173::entity::{EntityKind, EntityCategory};
use mc173::path::PathFinder;
use mc173::block;

use crate::world::{ServerWorldState, TickMode};
use crate::proto::{OutPacket, self};
use crate::player::ServerPlayer;


/// Describe all the context when a command is executed by something.
pub struct CommandContext<'a> {
    /// The command parts.
    pub parts: &'a [&'a str],
    /// The world to run the command in.
    pub world: &'a mut World,
    /// The server state associated to the world.
    pub state: &'a mut ServerWorldState,
    /// The dynamic reference to the command sender.
    pub player: &'a mut ServerPlayer,
}

/// Handle a command and execute it.
pub fn handle_command(ctx: CommandContext) {

    let Some(&cmd_name) = ctx.parts.first() else {
        ctx.player.send_chat(format!("§eNo command, type help!"));
        return;
    };

    for cmd in COMMANDS {
        if cmd.name == cmd_name {

            let res = (cmd.handler)(CommandContext { 
                parts: &ctx.parts[1..], 
                world: ctx.world, 
                state: ctx.state, 
                player: ctx.player,
            });

            match res {
                Err(Some(message)) => 
                    ctx.player.send_chat(message),
                Err(None) => 
                    ctx.player.send_chat(format!("§eUsage:§r /{} {}", cmd.name, cmd.usage)),
                _ => {}
            }

            return;

        }
    }

    ctx.player.send_chat(format!("§eUnknown command, type help!"));

}

/// The result of a command, if the result is ok, nothing is done, if the result is an
/// error, the optional message is printed, if no message is given the command usage
/// is displayed to the player.
type CommandResult = Result<(), Option<String>>;

/// Describe a command.
struct Command {
    /// The command name.
    name: &'static str,
    /// The command usage.
    usage: &'static str,
    /// The command description for help message.
    description: &'static str,
    /// The command handler to call when executing it.
    handler: fn(CommandContext) -> CommandResult,
}

/// Internal array of commands.
const COMMANDS: &'static [Command] = &[
    Command {
        name: "help",
        usage: "",
        description: "Print all available commands",
        handler: cmd_help
    },
    Command {
        name: "give",
        usage: "<item>[:<damage>] [<size>]",
        description: "Give item to a player",
        handler: cmd_give
    },
    Command {
        name: "spawn",
        usage: "<entity_kind> [<params>...]",
        description: "Spawn an entity",
        handler: cmd_spawn
    },
    Command {
        name: "time",
        usage: "",
        description: "Display world and server time",
        handler: cmd_time
    },
    Command {
        name: "weather",
        usage: "[clear|rain|thunder]",
        description: "Display world weather",
        handler: cmd_weather
    },
    Command {
        name: "pos",
        usage: "",
        description: "Display many information about current position",
        handler: cmd_pos
    },
    Command {
        name: "effect",
        usage: "<id> [<data>]",
        description: "Make some effect in the world",
        handler: cmd_effect
    },
    Command {
        name: "path",
        usage: "<x> <y> <z>",
        description: "Try to path find to a given position",
        handler: cmd_path
    },
    Command {
        name: "tick",
        usage: "freeze|auto|{step [n]}",
        description: "Control how the world is being ticked",
        handler: cmd_tick
    },
    Command {
        name: "clean",
        usage: "",
        description: "Remove all entity in the world except the player",
        handler: cmd_clean
    },
    Command {
        name: "explode",
        usage: "",
        description: "Make an explosion on the player position",
        handler: cmd_explode
    },
    Command {
        name: "perf",
        usage: "",
        description: "Display performance indicators for the current world",
        handler: cmd_perf,
    }
];

fn cmd_help(ctx: CommandContext) -> CommandResult {

    ctx.player.send_chat(format!("§8====================================================="));
    
    for cmd in COMMANDS {
        if cmd.usage.is_empty() {
            ctx.player.send_chat(format!("§a/{}:§r {}", cmd.name, cmd.description));
        } else {
            ctx.player.send_chat(format!("§a/{} {}:§r {}", cmd.name, cmd.usage, cmd.description));
        }
    }

    Ok(())
    
}

fn cmd_give(ctx: CommandContext) -> CommandResult {

    if ctx.parts.len() != 1 && ctx.parts.len() != 2 {
        return Err(None);
    }

    let item_raw = ctx.parts[0];

    let (
        id_raw, 
        metadata_raw
    ) = item_raw.split_once(':').unwrap_or((item_raw, ""));

    let id;
    if let Ok(direct_id) = id_raw.parse::<u16>() {
        id = direct_id;
    } else if let Some(name_id) = item::from_name(id_raw.trim_start_matches("i/")) {
        id = name_id;
    } else if let Some(block_id) = block::from_name(id_raw.trim_start_matches("b/")) {
        id = block_id as u16;
    } else {
        return Err(Some(format!("§cError: unknown item name or id:§r {id_raw}")));
    }

    let item = item::from_id(id);
    if item.name.is_empty() {
        return Err(Some(format!("§cError: unknown item id:§r {id_raw}")));
    }

    let mut stack = ItemStack::new_sized(id, 0, item.max_stack_size);

    if !metadata_raw.is_empty() {
        stack.damage = metadata_raw.parse::<u16>()
            .map_err(|_| format!("§cError: invalid item damage:§r {metadata_raw}"))?;
    }

    if let Some(size_raw) = ctx.parts.get(1) {
        stack.size = size_raw.parse::<u16>()
            .map_err(|_| format!("§cError: invalid stack size:§r {size_raw}"))?;
    }

    ctx.player.send_chat(format!("§aGiving §r{}§a (§r{}:{}§a) x§r{}§a to §r{}", item.name, stack.id, stack.damage, stack.size, ctx.player.username));
    ctx.player.pickup_stack(&mut stack);
    Ok(())

}

fn cmd_spawn(ctx: CommandContext) -> CommandResult {

    let [entity_kind_raw] = *ctx.parts else {
        return Err(None);
    };

    let entity_kind = match entity_kind_raw {
        "item" => EntityKind::Item,
        "boat" => EntityKind::Boat,
        "minecart" => EntityKind::Minecart,
        "pig" => EntityKind::Pig,
        "chicken" => EntityKind::Chicken,
        "cow" => EntityKind::Cow,
        "sheep" => EntityKind::Sheep,
        "zombie" => EntityKind::Zombie,
        "skeleton" => EntityKind::Skeleton,
        "ghast" => EntityKind::Ghast,
        "slime" => EntityKind::Slime,
        "creeper" => EntityKind::Creeper,
        "squid" => EntityKind::Squid,
        _ => return Err(Some(format!("§cError: invalid or unsupported entity kind:§r {entity_kind_raw}")))
    };

    let mut entity = entity_kind.new_default(ctx.player.pos);
    entity.0.persistent = true;

    entity.init_natural_spawn(ctx.world);

    let entity_id = ctx.world.spawn_entity(entity);
    ctx.player.send_chat(format!("§aEntity spawned:§r {entity_id}"));

    Ok(())

}

fn cmd_time(ctx: CommandContext) -> CommandResult {
    ctx.player.send_chat(format!("§aWorld time:§r {}", ctx.world.get_time()));
    ctx.player.send_chat(format!("§aServer time:§r {}", ctx.state.time));
    Ok(())
}

fn cmd_weather(ctx: CommandContext) -> CommandResult { 

    if ctx.parts.len() == 1 {
        
        let weather = match ctx.parts[0] {
            "clear" => Weather::Clear,
            "rain" => Weather::Rain,
            "thunder" => Weather::Thunder,
            _ => return Err(None)
        };

        ctx.world.set_weather(weather);
        ctx.player.send_chat(format!("§aWeather set to:§r {:?}", weather));
        Ok(())

    } else if ctx.parts.is_empty() {
        ctx.player.send_chat(format!("§aWeather:§r {:?}", ctx.world.get_weather()));
        Ok(())
    } else {
        Err(None)
    }

}

fn cmd_pos(ctx: CommandContext) -> CommandResult { 
    
    ctx.player.send_chat(format!("§8====================================================="));

    let block_pos = ctx.player.pos.floor().as_ivec3();
    ctx.player.send_chat(format!("§aReal:§r {}", ctx.player.pos));
    ctx.player.send_chat(format!("§aBlock:§r {}", block_pos));

    if let Some(height) = ctx.world.get_height(block_pos) {
        ctx.player.send_chat(format!("§aHeight:§r {}", height));
    }

    let light = ctx.world.get_light(block_pos);
    ctx.player.send_chat(format!("§aBlock light:§r {}", light.block));
    ctx.player.send_chat(format!("§aSky light:§r {}", light.sky));
    ctx.player.send_chat(format!("§aSky real light:§r {}", light.sky_real));
    ctx.player.send_chat(format!("§aBrightness:§r {}", light.brightness()));

    if let Some(biome) = ctx.world.get_biome(block_pos) {
        ctx.player.send_chat(format!("§aBiome:§r {biome:?}"));
    }

    Ok(())
    
}

fn cmd_effect(ctx: CommandContext) -> CommandResult { 

    if ctx.parts.len() != 1 && ctx.parts.len() != 2 {
        return Err(None);
    }

    let effect_raw = ctx.parts[0];
    let (effect_id, mut effect_data) = match effect_raw {
        "click" => (1000, 0),
        "click2" => (1001, 0),
        "bow" => (1002, 0),
        "door" => (1003, 0),
        "fizz" => (1004, 0),
        "record_13" => (1005, 2000),
        "record_cat" => (1005, 2001),
        "smoke" => (2000, 0),
        "break" => (2001, 0),
        _ => {
            let id = effect_raw.parse::<u32>()
                .map_err(|_| format!("§cError: invalid effect id:§r {effect_raw}"))?;
            (id, 0)
        }
    };

    if let Some(effect_data_raw) = ctx.parts.get(1) {
        effect_data = effect_data_raw.parse::<u32>()
            .map_err(|_| format!("§cError: invalid effect data:§r {effect_data_raw}"))?;
    }

    let pos = ctx.player.pos.floor().as_ivec3();
    ctx.player.send(OutPacket::EffectPlay(proto::EffectPlayPacket {
        x: pos.x,
        y: pos.y as i8,
        z: pos.z,
        effect_id,
        effect_data,
    }));

    ctx.player.send_chat(format!("§aPlayed effect:§r {effect_id}/{effect_data}"));
    Ok(())
    
}

fn cmd_path(ctx: CommandContext) -> CommandResult { 

    let [x_raw, y_raw, z_raw] = *ctx.parts else {
        return Err(None);
    };

    let from = ctx.player.pos.floor().as_ivec3();
    let to = IVec3 {
        x: x_raw.parse::<i32>().map_err(|_| format!("§cError: invalid x:§r {x_raw}"))?,
        y: y_raw.parse::<i32>().map_err(|_| format!("§cError: invalid y:§r {y_raw}"))?,
        z: z_raw.parse::<i32>().map_err(|_| format!("§cError: invalid z:§r {z_raw}"))?,
    };

    if let Some(path) = PathFinder::new(ctx.world).find_path(from, to, IVec3::ONE, 20.0) {
        
        for pos in path {
            ctx.world.set_block(pos, block::DEAD_BUSH, 0);
        }

        Ok(())

    } else {
        Err(Some(format!("§cError: path not found")))
    }

}

fn cmd_tick(ctx: CommandContext) -> CommandResult { 
    match ctx.parts {
        ["freeze"] => {
            ctx.player.send_chat(format!("§aWorld ticking:§r freeze"));
            ctx.state.tick_mode = TickMode::Manual(0);
            Ok(())
        }
        ["auto"] => {
            ctx.player.send_chat(format!("§aWorld ticking:§r auto"));
            ctx.state.tick_mode = TickMode::Auto;
            Ok(())
        }
        ["step"] => {
            ctx.player.send_chat(format!("§aWorld ticking:§r step"));
            ctx.state.tick_mode = TickMode::Manual(1);
            Ok(())
        }
        ["step", step_count] => {

            let step_count = step_count.parse::<u32>()
                .map_err(|_| format!("§cError: invalid step count:§r {step_count}"))?;

            ctx.player.send_chat(format!("§aWorld ticking:§r {step_count} steps"));
            ctx.state.tick_mode = TickMode::Manual(step_count);
            Ok(())

        }
        _ => return Err(None)
    }
}

fn cmd_clean(ctx: CommandContext) -> CommandResult { 

    let ids = ctx.world.iter_entities().map(|(id, _)| id).collect::<Vec<_>>();
    let mut removed_count = 0;
    for id in ids {
        if !ctx.world.is_entity_player(id) {
            assert!(ctx.world.remove_entity(id, "server clean command"));
            removed_count += 1;
        }
    }
    
    ctx.player.send_chat(format!("§aCleaned entities:§r {removed_count}"));
    Ok(())

}

fn cmd_explode(ctx: CommandContext) -> CommandResult { 

    ctx.world.explode(ctx.player.pos, 4.0, false, Some(ctx.player.entity_id));
    ctx.player.send_chat(format!("§aExplode at:§r {}", ctx.player.pos));
    Ok(())

}

fn cmd_perf(ctx: CommandContext) -> CommandResult { 

    ctx.player.send_chat(format!("§8====================================================="));
    ctx.player.send_chat(format!("§aTick duration:§r {:.1} ms", ctx.state.tick_duration.get() * 1000.0));
    ctx.player.send_chat(format!("§aTick interval:§r {:.1} ms", ctx.state.tick_interval.get() * 1000.0));
    ctx.player.send_chat(format!("§aEvents:§r {:.1} ({:.1} kB)", ctx.state.events_count.get(), ctx.state.events_count.get() * mem::size_of::<Event>() as f32 / 1000.0));
    
    ctx.player.send_chat(format!("§aEntities:§r {} ({} players)", ctx.world.get_entity_count(), ctx.world.get_entity_player_count()));
    
    let mut categories_count = [0usize; EntityCategory::ALL.len()];
    for (_, entity) in ctx.world.iter_entities() {
        categories_count[entity.category() as usize] += 1;
    }
    
    for category in EntityCategory::ALL {
        ctx.player.send_chat(format!("  §a{category:?}s:§r {}", categories_count[category as usize]));
    }

    ctx.player.send_chat(format!("§aBlock ticks:§r {}", ctx.world.get_block_tick_count()));
    ctx.player.send_chat(format!("§aLight updates:§r {}", ctx.world.get_light_update_count()));

    Ok(())

}
