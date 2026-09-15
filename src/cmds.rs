//! Edition-specific Minecraft command printers.
//!
//! Bedrock and Java do not share an item grammar, text format, or several
//! command shapes (`effect give` vs `effect`, `item replace` vs `replaceitem`,
//! `distance=` vs `r=`). Every helper here takes [`Edition`] and returns **one**
//! legal line with no leading `/`.

use crate::config::Edition;

/// Compile-time item stack (id + count + edition extras). Not a scoreboard.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemStack {
    pub id: String,
    pub count: i64,
    pub data: Option<i64>,
    pub components: Vec<(String, String)>,
}

impl ItemStack {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: normalize_item_id(&id.into()),
            count: 1,
            data: None,
            components: Vec::new(),
        }
    }

    pub fn with_count(mut self, count: i64) -> Self {
        self.count = count.max(1);
        self
    }

    /// Item id as written in a command (Java prefers a namespace).
    pub fn cmd_id(&self, edition: Edition) -> String {
        namespaced_id(&self.id, edition)
    }

    /// Java `/give` item argument, including `[components]` when present.
    pub fn java_stack(&self) -> String {
        let id = namespaced_id(&self.id, Edition::Java);
        if self.components.is_empty() {
            id
        } else {
            let patch = self
                .components
                .iter()
                .map(|(k, v)| {
                    if v.is_empty() {
                        k.clone()
                    } else {
                        format!("{k}={v}")
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{id}[{patch}]")
        }
    }

    fn bedrock_components_json(&self) -> Option<String> {
        if self.components.is_empty() {
            return None;
        }
        let mut parts = Vec::new();
        for (k, v) in &self.components {
            let key = if k.contains(':') {
                k.clone()
            } else {
                format!("minecraft:{k}")
            };
            let val = v.trim();
            if val.starts_with('{') || val.starts_with('[') || val.starts_with('"') {
                parts.push(format!("\"{key}\":{val}"));
            } else {
                parts.push(format!("\"{key}\":\"{}\"", escape_json(val)));
            }
        }
        Some(format!("{{{}}}", parts.join(",")))
    }
}

fn normalize_item_id(id: &str) -> String {
    let id = id.trim().trim_start_matches("minecraft:");
    id.replace('-', "_").to_lowercase()
}

fn namespaced_id(id: &str, edition: Edition) -> String {
    let id = normalize_item_id(id);
    match edition {
        Edition::Java if !id.contains(':') => format!("minecraft:{id}"),
        _ => id,
    }
}

fn effect_id(id: &str, edition: Edition) -> String {
    let id = id.trim().trim_start_matches("minecraft:").to_lowercase();
    match edition {
        Edition::Java => format!("minecraft:{id}"),
        Edition::Bedrock => id,
    }
}

fn sound_id(id: &str) -> String {
    id.trim().to_string()
}

/// Strip a leading `/` so function/chain bodies stay slash-free.
pub fn strip_slash(cmd: &str) -> String {
    cmd.trim().trim_start_matches('/').trim().to_string()
}

pub fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn java_slot(slot: &str) -> String {
    slot.trim()
        .trim_start_matches("slot.")
        .trim()
        .to_string()
}

fn bedrock_slot(slot: &str) -> String {
    let s = slot.trim();
    if s.starts_with("slot.") {
        s.to_string()
    } else {
        format!("slot.{s}")
    }
}

/// `give <selector> <item> …`
pub fn give(edition: Edition, selector: &str, item: &ItemStack) -> Result<String, String> {
    let count = item.count.max(1);
    match edition {
        Edition::Bedrock => {
            if item
                .components
                .iter()
                .any(|(k, _)| k.contains('[') || k.contains(']'))
            {
                return Err("Java item components `item[...]` are not valid on Bedrock".into());
            }
            let id = item.cmd_id(edition);
            let mut cmd = format!("give {selector} {id} {count}");
            let json = item.bedrock_components_json();
            if let Some(data) = item.data {
                cmd.push_str(&format!(" {data}"));
            } else if json.is_some() {
                cmd.push_str(" 0");
            }
            if let Some(json) = json {
                cmd.push(' ');
                cmd.push_str(&json);
            }
            Ok(cmd)
        }
        Edition::Java => {
            if item.data.is_some() {
                return Err(
                    "`Item.data(...)` is Bedrock aux-value syntax; invalid on Java".into(),
                );
            }
            Ok(format!(
                "give {selector} {} {count}",
                item.java_stack()
            ))
        }
    }
}

/// `kill <selector>`
pub fn kill(edition: Edition, selector: &str) -> Result<String, String> {
    let _ = edition;
    Ok(format!("kill {selector}"))
}

/// `effect` — Java uses `effect give` / `effect clear`; Bedrock does not.
pub fn effect_give(
    edition: Edition,
    selector: &str,
    effect: &str,
    seconds: i64,
    amplifier: i64,
    hide_particles: bool,
) -> Result<String, String> {
    let hide = if hide_particles { "true" } else { "false" };
    let id = effect_id(effect, edition);
    match edition {
        Edition::Java => Ok(format!(
            "effect give {selector} {id} {seconds} {amplifier} {hide}"
        )),
        Edition::Bedrock => Ok(format!(
            "effect {selector} {id} {seconds} {amplifier} {hide}"
        )),
    }
}

pub fn effect_clear(
    edition: Edition,
    selector: &str,
    effect: Option<&str>,
) -> Result<String, String> {
    match edition {
        Edition::Java => match effect {
            Some(e) if !e.is_empty() && e != "clear" => {
                Ok(format!("effect clear {selector} {}", effect_id(e, edition)))
            }
            _ => Ok(format!("effect clear {selector}")),
        },
        Edition::Bedrock => {
            if let Some(e) = effect {
                if !e.is_empty() && e != "clear" {
                    return Err(
                        "Bedrock `/effect` clears all effects with `effect <player> clear`"
                            .into(),
                    );
                }
            }
            Ok(format!("effect {selector} clear"))
        }
    }
}

/// `clear <selector> [item] [maxCount]` — Bedrock also takes aux `data`.
pub fn clear(
    edition: Edition,
    selector: &str,
    item: Option<&ItemStack>,
    max_count: Option<i64>,
) -> Result<String, String> {
    match edition {
        Edition::Java => {
            let mut cmd = format!("clear {selector}");
            if let Some(item) = item {
                if item.data.is_some() {
                    return Err("`clear` aux `data` is Bedrock-only".into());
                }
                cmd.push(' ');
                cmd.push_str(&item.java_stack());
                if let Some(n) = max_count {
                    cmd.push_str(&format!(" {n}"));
                }
            }
            Ok(cmd)
        }
        Edition::Bedrock => {
            let mut cmd = format!("clear {selector}");
            if let Some(item) = item {
                cmd.push(' ');
                cmd.push_str(&item.cmd_id(edition));
                let data = item.data.unwrap_or(-1);
                cmd.push_str(&format!(" {data}"));
                if let Some(n) = max_count {
                    cmd.push_str(&format!(" {n}"));
                }
            }
            Ok(cmd)
        }
    }
}

/// `playsound` — Java requires a source category; Bedrock does not.
pub fn playsound(
    edition: Edition,
    sound: &str,
    selector: &str,
    pos: Option<&str>,
    volume: Option<f64>,
    pitch: Option<f64>,
    source: Option<&str>,
) -> Result<String, String> {
    let sound = sound_id(sound);
    match edition {
        Edition::Java => {
            let src = source.unwrap_or("master");
            let mut cmd = format!("playsound {sound} {src} {selector}");
            if let Some(pos) = pos {
                cmd.push(' ');
                cmd.push_str(pos);
                if let Some(v) = volume {
                    cmd.push_str(&format!(" {v}"));
                    if let Some(p) = pitch {
                        cmd.push_str(&format!(" {p}"));
                    }
                }
            }
            Ok(cmd)
        }
        Edition::Bedrock => {
            let mut cmd = format!("playsound {sound} {selector}");
            if let Some(pos) = pos {
                cmd.push(' ');
                cmd.push_str(pos);
                if let Some(v) = volume {
                    cmd.push_str(&format!(" {v}"));
                    if let Some(p) = pitch {
                        cmd.push_str(&format!(" {p}"));
                    }
                }
            }
            Ok(cmd)
        }
    }
}

pub fn particle(edition: Edition, name: &str, pos: &str) -> Result<String, String> {
    let _ = edition;
    Ok(format!("particle {name} {pos}"))
}

/// `summon` — Java may take SNBT; Bedrock takes spawnEvent / nameTag, never NBT.
pub fn summon(
    edition: Edition,
    entity: &str,
    pos: &str,
    extra: Option<&str>,
) -> Result<String, String> {
    let id = namespaced_id(entity, edition);
    match edition {
        Edition::Java => match extra {
            Some(nbt) if !nbt.is_empty() => Ok(format!("summon {id} {pos} {nbt}")),
            _ => Ok(format!("summon {id} {pos}")),
        },
        Edition::Bedrock => {
            if extra.is_some_and(|s| s.trim().starts_with('{')) {
                return Err("Bedrock `/summon` does not take Java SNBT; omit NBT".into());
            }
            match extra {
                Some(ev) if !ev.is_empty() => Ok(format!("summon {id} {pos} {ev}")),
                _ => Ok(format!("summon {id} {pos}")),
            }
        }
    }
}

pub fn setblock(edition: Edition, pos: &str, block: &str) -> Result<String, String> {
    match edition {
        Edition::Java => Ok(format!("setblock {pos} {}", namespaced_id(block, edition))),
        Edition::Bedrock => {
            if block.contains('[') {
                return Err(
                    "Java block states `id[key=value]` are not Bedrock `setblock` syntax".into(),
                );
            }
            Ok(format!("setblock {pos} {}", namespaced_id(block, edition)))
        }
    }
}

pub fn title(
    edition: Edition,
    selector: &str,
    location: &str,
    text: &str,
) -> Result<String, String> {
    let loc = match location.to_lowercase().as_str() {
        "title" | "subtitle" | "actionbar" | "times" | "clear" | "reset" => {
            location.to_lowercase()
        }
        _ => "title".into(),
    };
    match loc.as_str() {
        "clear" | "reset" => Ok(format!("title {selector} {loc}")),
        "times" => Ok(format!("title {selector} times {text}")),
        _ => match edition {
            Edition::Bedrock => Ok(format!("title {selector} {loc} {text}")),
            Edition::Java => Ok(format!(
                "title {selector} {loc} {{\"text\":\"{}\"}}",
                escape_json(text)
            )),
        },
    }
}

pub fn tellraw(edition: Edition, selector: &str, text: &str) -> Result<String, String> {
    match edition {
        Edition::Bedrock => Ok(format!(
            "tellraw {selector} {{\"rawtext\":[{{\"text\":\"{}\"}}]}}",
            escape_json(text)
        )),
        Edition::Java => Ok(format!(
            "tellraw {selector} {{\"text\":\"{}\"}}",
            escape_json(text)
        )),
    }
}

pub fn say(text: &str) -> String {
    format!("say {text}")
}

pub fn weather(kind: &str, duration: Option<i64>) -> String {
    let kind = kind.to_lowercase();
    match duration {
        Some(d) => format!("weather {kind} {d}"),
        None => format!("weather {kind}"),
    }
}

pub fn time_set(spec: &str) -> String {
    format!("time set {spec}")
}

pub fn difficulty(kind: &str) -> String {
    format!("difficulty {}", kind.to_lowercase())
}

pub fn gamemode(mode: &str, selector: &str) -> String {
    format!("gamemode {} {selector}", mode.to_lowercase())
}

pub fn teleport(selector: &str, pos: &str) -> String {
    format!("teleport {selector} {pos}")
}

/// Experience: Java `xp add`; Bedrock `xp <amount> [player]` / `xp <amount>L`.
pub fn xp(
    edition: Edition,
    selector: &str,
    amount: i64,
    levels: bool,
) -> Result<String, String> {
    match edition {
        Edition::Java => {
            let unit = if levels { "levels" } else { "points" };
            Ok(format!("xp add {selector} {amount} {unit}"))
        }
        Edition::Bedrock => {
            if levels {
                Ok(format!("xp {amount}L {selector}"))
            } else {
                Ok(format!("xp {amount} {selector}"))
            }
        }
    }
}

pub fn enchant(
    edition: Edition,
    selector: &str,
    enchant: &str,
    level: i64,
) -> Result<String, String> {
    let id = namespaced_id(enchant, edition);
    Ok(format!("enchant {selector} {id} {level}"))
}

/// Bedrock `replaceitem` / Java `item replace`.
pub fn replace_item(
    edition: Edition,
    selector: &str,
    slot: &str,
    item: &ItemStack,
) -> Result<String, String> {
    let count = item.count.max(1);
    match edition {
        Edition::Bedrock => {
            let mut cmd = format!(
                "replaceitem entity {selector} {} {} {count}",
                bedrock_slot(slot),
                item.cmd_id(edition)
            );
            if let Some(data) = item.data {
                cmd.push_str(&format!(" {data}"));
            }
            Ok(cmd)
        }
        Edition::Java => {
            if item.data.is_some() {
                return Err("`replaceitem` aux `data` is Bedrock-only; Java uses `/item`".into());
            }
            Ok(format!(
                "item replace entity {selector} {} with {} {count}",
                java_slot(slot),
                item.java_stack()
            ))
        }
    }
}

pub fn gamerule(key: &str, value: &str) -> String {
    format!("gamerule {key} {value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn give_splits_editions() {
        let item = ItemStack::new("gold_ingot").with_count(2);
        assert_eq!(
            give(Edition::Bedrock, "@a", &item).unwrap(),
            "give @a gold_ingot 2"
        );
        assert_eq!(
            give(Edition::Java, "@a", &item).unwrap(),
            "give @a minecraft:gold_ingot 2"
        );
    }

    #[test]
    fn give_java_rejects_aux_data() {
        let mut item = ItemStack::new("potion");
        item.data = Some(7);
        assert!(give(Edition::Java, "@p", &item).is_err());
        assert!(give(Edition::Bedrock, "@p", &item)
            .unwrap()
            .ends_with(" 7"));
    }

    #[test]
    fn give_java_components_use_brackets() {
        let mut item = ItemStack::new("diamond_sword");
        item.components
            .push(("enchantments".into(), "{levels:{\"minecraft:sharpness\":5}}".into()));
        let cmd = give(Edition::Java, "@s", &item).unwrap();
        assert!(cmd.contains("diamond_sword[enchantments="), "{cmd}");
        assert!(!cmd.contains("hasitem="), "{cmd}");
    }

    #[test]
    fn effect_and_xp_and_item_slot_split() {
        assert_eq!(
            effect_give(Edition::Java, "@s", "speed", 10, 1, true).unwrap(),
            "effect give @s minecraft:speed 10 1 true"
        );
        assert_eq!(
            effect_give(Edition::Bedrock, "@s", "speed", 10, 1, true).unwrap(),
            "effect @s speed 10 1 true"
        );
        assert_eq!(
            effect_clear(Edition::Java, "@a", None).unwrap(),
            "effect clear @a"
        );
        assert_eq!(
            effect_clear(Edition::Bedrock, "@a", None).unwrap(),
            "effect @a clear"
        );
        assert_eq!(
            xp(Edition::Java, "@s", 5, false).unwrap(),
            "xp add @s 5 points"
        );
        assert_eq!(xp(Edition::Bedrock, "@s", 5, false).unwrap(), "xp 5 @s");
        assert_eq!(xp(Edition::Bedrock, "@s", 5, true).unwrap(), "xp 5L @s");

        let item = ItemStack::new("iron_ingot");
        assert_eq!(
            replace_item(Edition::Bedrock, "@s", "hotbar.0", &item).unwrap(),
            "replaceitem entity @s slot.hotbar.0 iron_ingot 1"
        );
        assert_eq!(
            replace_item(Edition::Java, "@s", "slot.hotbar.0", &item).unwrap(),
            "item replace entity @s hotbar.0 with minecraft:iron_ingot 1"
        );
    }

    #[test]
    fn tellraw_and_title_split() {
        assert!(tellraw(Edition::Bedrock, "@a", "hi")
            .unwrap()
            .contains("rawtext"));
        assert!(tellraw(Edition::Java, "@a", "hi")
            .unwrap()
            .contains("{\"text\":\"hi\"}"));
        assert_eq!(
            title(Edition::Bedrock, "@a", "title", "地铁").unwrap(),
            "title @a title 地铁"
        );
        assert!(title(Edition::Java, "@a", "title", "地铁")
            .unwrap()
            .contains("{\"text\":\"地铁\"}"));
    }

    #[test]
    fn playsound_java_has_source() {
        let j = playsound(
            Edition::Java,
            "minecraft:entity.player.levelup",
            "@a",
            Some("~ ~ ~"),
            Some(1.0),
            None,
            None,
        )
        .unwrap();
        assert!(j.contains("master"), "{j}");
        let b = playsound(
            Edition::Bedrock,
            "random.levelup",
            "@a",
            Some("~ ~ ~"),
            Some(1.0),
            None,
            None,
        )
        .unwrap();
        assert!(!b.contains("master"), "{b}");
    }

    #[test]
    fn bedrock_summon_rejects_snbt() {
        assert!(summon(Edition::Bedrock, "zombie", "0 64 0", Some("{NoAI:1b}")).is_err());
        assert_eq!(
            summon(Edition::Java, "zombie", "0 64 0", Some("{NoAI:1b}")).unwrap(),
            "summon minecraft:zombie 0 64 0 {NoAI:1b}"
        );
    }
}
