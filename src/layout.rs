//! Command-block layouts: linear, stack, snake, box, points.
//! Facing values match Minecraft: 0 down, 1 up, 2 north, 3 south, 4 west, 5 east.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}

impl Facing {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "down" => Facing::Down,
            "up" => Facing::Up,
            "north" => Facing::North,
            "south" => Facing::South,
            "west" => Facing::West,
            "east" => Facing::East,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Facing::Down => "down",
            Facing::Up => "up",
            Facing::North => "north",
            Facing::South => "south",
            Facing::West => "west",
            Facing::East => "east",
        }
    }

    pub fn delta(self) -> [i32; 3] {
        match self {
            Facing::Down => [0, -1, 0],
            Facing::Up => [0, 1, 0],
            Facing::North => [0, 0, -1],
            Facing::South => [0, 0, 1],
            Facing::West => [-1, 0, 0],
            Facing::East => [1, 0, 0],
        }
    }

    pub fn reverse(self) -> Self {
        match self {
            Facing::Down => Facing::Up,
            Facing::Up => Facing::Down,
            Facing::North => Facing::South,
            Facing::South => Facing::North,
            Facing::West => Facing::East,
            Facing::East => Facing::West,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CbInstance {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub facing: Facing,
    pub mode: u8,
    pub flags: u8,
    pub delay_ticks: u32,
    pub command: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LayoutRequest {
    pub layout: String,
    pub facing: Facing,
    pub origin: [i32; 3],
    pub max_span: u32,
    pub bound: Option<[i32; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainCmdMeta {
    pub conditional: bool,
    pub delay: u32,
    pub impulse: bool,
    pub repeat: bool,
    pub always_active: bool,
    pub at: Option<[i32; 3]>,
    pub label: Option<String>,
}

impl Default for ChainCmdMeta {
    fn default() -> Self {
        Self {
            conditional: false,
            delay: 0,
            impulse: false,
            repeat: false,
            always_active: true,
            at: None,
            label: None,
        }
    }
}

/// Place `commands` along `layout`. First command is the clock/impulse head.
pub fn place_commands(
    origin: [i32; 3],
    layout: &str,
    facing: Facing,
    max_span: u32,
    bound: Option<[i32; 3]>,
    commands: &[(String, ChainCmdMeta)],
    clock: bool,
) -> Result<Vec<CbInstance>, String> {
    if commands.is_empty() {
        return Ok(Vec::new());
    }
    let n = commands.len();
    let coords = match layout {
        "linear" => linear(origin, facing, n),
        "stack" => stack(origin, facing, n),
        "snake" => snake(origin, facing, max_span.max(1), n),
        "box" => box_fill(
            origin,
            facing,
            bound.unwrap_or([max_span as i32, n as i32, 1]),
            n,
        )?,
        "points" => commands
            .iter()
            .enumerate()
            .map(|(i, (_, meta))| {
                meta.at
                    .unwrap_or([origin[0], origin[1] + i as i32, origin[2]])
            })
            .collect(),
        other => return Err(format!("unknown layout `{other}`")),
    };
    if coords.len() != n {
        return Err("layout produced the wrong number of positions".into());
    }
    let mut out = Vec::new();
    for (i, ((cmd, meta), [x, y, z])) in commands.iter().zip(coords).enumerate() {
        let mut mode = if meta.repeat {
            2
        } else if meta.impulse {
            0
        } else if i == 0 && clock {
            2
        } else if i == 0 {
            0
        } else {
            1
        };
        if i > 0 && mode == 2 {
            mode = 1;
        }
        if i == 0 && clock {
            mode = 2;
        }
        let mut flags = 0u8;
        if meta.conditional {
            flags |= 1;
        }
        if meta.always_active || clock && i == 0 {
            flags |= 2;
        }
        if i + 1 == n {
            flags |= 4;
        }
        let face = step_facing(layout, facing, i, max_span);
        out.push(CbInstance {
            x,
            y,
            z,
            facing: face,
            mode,
            flags,
            delay_ticks: meta.delay,
            command: cmd.clone(),
            label: meta.label.clone(),
        });
    }
    Ok(out)
}

fn step_facing(layout: &str, facing: Facing, i: usize, max_span: u32) -> Facing {
    match layout {
        "snake" => {
            let span = max_span.max(1) as usize;
            let row = i / span;
            if row % 2 == 0 {
                facing
            } else {
                facing.reverse()
            }
        }
        _ => facing,
    }
}

fn linear(origin: [i32; 3], facing: Facing, n: usize) -> Vec<[i32; 3]> {
    let d = facing.delta();
    (0..n)
        .map(|i| {
            let i = i as i32;
            [
                origin[0] + d[0] * i,
                origin[1] + d[1] * i,
                origin[2] + d[2] * i,
            ]
        })
        .collect()
}

fn stack(origin: [i32; 3], facing: Facing, n: usize) -> Vec<[i32; 3]> {
    let axis = match facing {
        Facing::Down | Facing::Up => facing,
        _ => Facing::Up,
    };
    linear(origin, axis, n)
}

fn snake(origin: [i32; 3], facing: Facing, max_span: u32, n: usize) -> Vec<[i32; 3]> {
    let span = max_span.max(1) as i32;
    let along = facing.delta();
    let step = match facing {
        Facing::East | Facing::West => [0, 1, 0],
        Facing::North | Facing::South => [0, 1, 0],
        Facing::Up | Facing::Down => [1, 0, 0],
    };
    let mut out = Vec::new();
    for i in 0..n {
        let i = i as i32;
        let row = i / span;
        let col = i % span;
        let col_dir: i32 = if row % 2 == 0 { 1 } else { -1 };
        let col_off = if row % 2 == 0 { col } else { span - 1 - col };
        out.push([
            origin[0] + along[0] * col_off * col_dir.abs() + step[0] * row,
            origin[1] + along[1] * col_off + step[1] * row,
            origin[2] + along[2] * col_off * col_dir.abs() + step[2] * row,
        ]);
        let _ = col_dir;
        let last = out.last_mut().unwrap();
        if row % 2 == 0 {
            *last = [
                origin[0] + along[0] * col + step[0] * row,
                origin[1] + along[1] * col + step[1] * row,
                origin[2] + along[2] * col + step[2] * row,
            ];
        } else {
            let back = span - 1 - col;
            *last = [
                origin[0] + along[0] * back + step[0] * row,
                origin[1] + along[1] * back + step[1] * row,
                origin[2] + along[2] * back + step[2] * row,
            ];
        }
    }
    out
}

fn box_fill(
    origin: [i32; 3],
    _facing: Facing,
    bound: [i32; 3],
    n: usize,
) -> Result<Vec<[i32; 3]>, String> {
    let sx = bound[0].max(1);
    let sy = bound[1].max(1);
    let sz = bound[2].max(1);
    let cap = (sx * sy * sz) as usize;
    if n > cap {
        return Err(format!(
            "box bound {sx}x{sy}x{sz} holds {cap} blocks, need {n}"
        ));
    }
    let mut out = Vec::new();
    for x in 0..sx {
        for z in 0..sz {
            for y in 0..sy {
                if out.len() == n {
                    return Ok(out);
                }
                out.push([origin[0] + x, origin[1] + y, origin[2] + z]);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_up_four() {
        let cmds: Vec<(String, ChainCmdMeta)> = (0..4)
            .map(|i| (format!("say {i}"), ChainCmdMeta::default()))
            .collect();
        let placed =
            place_commands([2, 64, 0], "stack", Facing::Up, 32, None, &cmds, true).unwrap();
        assert_eq!(placed[0].x, 2);
        assert_eq!(placed[0].y, 64);
        assert_eq!(placed[3].y, 67);
        assert_eq!(placed[0].mode, 2);
        assert_eq!(placed[1].mode, 1);
    }

    #[test]
    fn linear_east() {
        let cmds: Vec<(String, ChainCmdMeta)> = (0..3)
            .map(|i| (format!("say {i}"), ChainCmdMeta::default()))
            .collect();
        let placed =
            place_commands([0, 70, 0], "linear", Facing::East, 32, None, &cmds, false).unwrap();
        assert_eq!(placed[2].x, 2);
        assert_eq!(placed[0].facing, Facing::East);
    }
}
