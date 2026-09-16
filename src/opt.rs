//! Semantic peephole: the compiler does **not** emit 1:1 with source.
//! Commands are rewritten so the in-game effect is the same, with fewer
//! `execute` layers and no dead score ops.

/// Optimize a function/chain command list (no leading `/`).
pub fn optimize(cmds: Vec<String>) -> Vec<String> {
    let cmds: Vec<String> = cmds
        .into_iter()
        .map(|c| strip_identity_execute(&c))
        .filter(|c| !c.is_empty() && !c.starts_with('#'))
        .collect();
    let cmds = drop_redundant_add0(cmds);
    let cmds = coalesce_set_add(cmds);
    dedup_idempotent(cmds)
}

pub fn optimize_pairs<T: Clone>(cmds: Vec<(String, T)>) -> Vec<(String, T)> {
    let mut out: Vec<(String, T)> = Vec::with_capacity(cmds.len());
    for (cmd, meta) in cmds {
        let cmd = strip_identity_execute(&cmd);
        if cmd.is_empty() || cmd.starts_with('#') {
            continue;
        }
        if let Some((prev, _)) = out.last() {
            if prev == &cmd && is_idempotent(&cmd) {
                continue;
            }
        }
        if let Some(rest) = cmd.strip_prefix("scoreboard players add ") {
            if rest.ends_with(" 0") {
                let body = rest.trim_end_matches(" 0");
                let already = out.iter().any(|(c, _)| {
                    c.contains(body)
                        && (c.starts_with("scoreboard players set ")
                            || c.starts_with("scoreboard players operation "))
                });
                if already {
                    continue;
                }
            }
        }
        if let Some((prev, _)) = out.last_mut() {
            if let Some(folded) = fold_set_add(prev, &cmd) {
                *prev = folded;
                continue;
            }
        }
        out.push((cmd, meta));
    }
    out
}

fn strip_identity_execute(cmd: &str) -> String {
    let mut c = cmd.trim().to_string();
    loop {
        let next = if let Some(rest) = c.strip_prefix("execute run ") {
            rest.to_string()
        } else if let Some(rest) = c.strip_prefix("execute as @s run ") {
            rest.to_string()
        } else if let Some(rest) = c.strip_prefix("execute as @s ") {
            format!("execute {rest}")
        } else {
            break;
        };
        if next == c {
            break;
        }
        c = next;
    }
    c
}

fn drop_redundant_add0(cmds: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(cmds.len());
    for cmd in cmds {
        if let Some(rest) = cmd.strip_prefix("scoreboard players add ") {
            if rest.ends_with(" 0") {
                // `add … 0` only exists to create a score. Skip if we already
                // set/operated that holder+objective in this list.
                let body = rest.trim_end_matches(" 0");
                let already = out.iter().any(|c: &String| {
                    c.contains(body)
                        && (c.starts_with("scoreboard players set ")
                            || c.starts_with("scoreboard players operation "))
                });
                if already {
                    continue;
                }
            }
        }
        out.push(cmd);
    }
    out
}

fn coalesce_set_add(cmds: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(cmds.len());
    for cmd in cmds {
        if let Some(last) = out.last_mut() {
            if let Some(folded) = fold_set_add(last, &cmd) {
                *last = folded;
                continue;
            }
        }
        out.push(cmd);
    }
    out
}

fn fold_set_add(prev: &str, cmd: &str) -> Option<String> {
    let (h, o, n) = parse_score_set(prev)?;
    let (h2, o2, m, add) = parse_score_add(cmd)?;
    if h != h2 || o != o2 {
        return None;
    }
    let next = if add {
        n.saturating_add(m)
    } else {
        n.saturating_sub(m)
    };
    Some(format!("scoreboard players set {h} {o} {next}"))
}

fn parse_score_set(cmd: &str) -> Option<(String, String, i64)> {
    let rest = cmd.strip_prefix("scoreboard players set ")?;
    let mut parts = rest.rsplitn(2, ' ');
    let n: i64 = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    let (h, o) = rest.rsplit_once(' ')?;
    Some((h.to_string(), o.to_string(), n))
}

fn parse_score_add(cmd: &str) -> Option<(String, String, i64, bool)> {
    let (rest, add) = if let Some(r) = cmd.strip_prefix("scoreboard players add ") {
        (r, true)
    } else if let Some(r) = cmd.strip_prefix("scoreboard players remove ") {
        (r, false)
    } else {
        return None;
    };
    let mut parts = rest.rsplitn(2, ' ');
    let n: i64 = parts.next()?.parse().ok()?;
    let rest = parts.next()?;
    let (h, o) = rest.rsplit_once(' ')?;
    Some((h.to_string(), o.to_string(), n, add))
}

fn is_idempotent(cmd: &str) -> bool {
    cmd.starts_with("scoreboard players set ")
        || cmd.starts_with("scoreboard objectives add ")
        || (cmd.starts_with("tag ") && cmd.contains(" add "))
        || cmd.starts_with("gamerule ")
}

fn dedup_idempotent(cmds: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(cmds.len());
    for cmd in cmds {
        if out.last().is_some_and(|p| p == &cmd && is_idempotent(&cmd)) {
            continue;
        }
        out.push(cmd);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_execute_run_and_as_s() {
        assert_eq!(optimize(vec!["execute run say hi".into()]), vec!["say hi"]);
        assert_eq!(
            optimize(vec!["execute as @s run say hi".into()]),
            vec!["say hi"]
        );
        assert_eq!(
            optimize(vec!["execute as @s at @s run say hi".into()]),
            vec!["execute at @s run say hi"]
        );
    }

    #[test]
    fn drops_dup_and_add0_after_set() {
        let out = optimize(vec![
            "scoreboard players set mcs m00 3".into(),
            "scoreboard players add mcs m00 0".into(),
            "scoreboard players set mcs m00 3".into(),
        ]);
        assert_eq!(out, vec!["scoreboard players set mcs m00 3".to_string()]);
    }

    #[test]
    fn does_not_collapse_repeated_say() {
        let out = optimize(vec!["say hi".into(), "say hi".into(), "say hi".into()]);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn folds_set_then_add_into_one_set() {
        let out = optimize(vec![
            "scoreboard players set mcs m00 3".into(),
            "scoreboard players add mcs m00 1".into(),
        ]);
        assert_eq!(out, vec!["scoreboard players set mcs m00 4".to_string()]);
    }
}
