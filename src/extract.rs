use crate::ast::{
    CompilationUnit, ContainerKind, Expr, FieldDef, Item, Member, PlaceStmt, TypeRef, UnaryOp,
    WorldClause,
};

/// MINCB-facing facts extracted from a parsed compilation unit (or merged project).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryInfo {
    pub pack: String,
    pub score_revision: u32,
    pub symbols: Vec<Symbol>,
    pub chains: Vec<ChainInfo>,
    pub blocks: Vec<PlacedBlock>,
    pub fills: Vec<FillOp>,
    pub containers: Vec<ContainerInfo>,
    pub links: Vec<LinkInfo>,
    pub origin: Option<[i64; 3]>,
    pub dimension: Option<String>,
    pub ticking_areas: Vec<TickingAreaInfo>,
    pub clock: Option<String>,
    pub host_tick: Option<String>,
    pub functions: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub id: String,
    pub kind: SymbolKind,
    pub qualified: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Objective = 0,
    Tag = 1,
    FakePlayer = 2,
    Function = 3,
    Chain = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainInfo {
    pub name: String,
    pub annotations: Vec<String>,
    pub layout: Option<String>,
    pub facing: Option<String>,
    pub origin: Option<[i64; 3]>,
    pub absolute: bool,
    pub bound: Option<[i64; 3]>,
    pub is_clock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedBlock {
    pub block: String,
    pub at: [i64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillOp {
    pub from: [i64; 3],
    pub to: [i64; 3],
    pub block: String,
    pub replace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInfo {
    pub kind: ContainerKind,
    pub name: String,
    pub at: [i64; 3],
    pub facing: Option<String>,
    pub slots: Vec<SlotInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotInfo {
    pub slot: i64,
    pub item: String,
    pub count: i64,
    pub data: Option<i64>,
    pub title: Option<String>,
    pub pages: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickingAreaInfo {
    pub name: String,
    pub center: Option<[i64; 3]>,
    pub radius: Option<i64>,
    pub preload: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkInfo {
    pub from_chain: String,
    pub from_label: Option<String>,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInfo {
    pub class: String,
    pub name: String,
    pub annotations: Vec<String>,
    pub is_static: bool,
}

/// Assign stable short ids (`m00`, `t0a`, …) for scores and tags.
pub fn extract_binary_info(unit: &CompilationUnit) -> BinaryInfo {
    extract_binary_info_with_revision(unit, 1)
}

pub fn extract_binary_info_with_revision(
    unit: &CompilationUnit,
    score_revision: u32,
) -> BinaryInfo {
    let pack = unit.pack.dotted();
    let mut field_names: Vec<(String, SymbolKind)> = Vec::new();
    let mut functions = Vec::new();

    for item in &unit.items {
        if let Item::Class(class) = item {
            for member in &class.members {
                match member {
                    Member::Field(field) => {
                        if let Some(kind) = kind_of_field(field) {
                            let qualified = format!("{pack}.{}.{}", class.name, field.name);
                            field_names.push((qualified, kind));
                        }
                    }
                    Member::Method(method) => {
                        functions.push(FunctionInfo {
                            class: class.name.clone(),
                            name: method.name.clone(),
                            annotations: method
                                .annotations
                                .iter()
                                .map(|a| a.name.clone())
                                .collect(),
                            is_static: method.is_static,
                        });
                    }
                }
            }
        }
    }
    field_names.sort_by(|a, b| a.0.cmp(&b.0));

    let mut obj_i = 0u32;
    let mut tag_i = 0u32;
    let mut symbols = Vec::new();
    symbols.push(Symbol {
        id: "mcs".into(),
        kind: SymbolKind::FakePlayer,
        qualified: format!("{pack}.#mcs"),
    });
    symbols.push(Symbol {
        id: "mt".into(),
        kind: SymbolKind::Objective,
        qualified: format!("{pack}.#temps"),
    });
    for (qualified, kind) in field_names {
        let id = match kind {
            SymbolKind::Objective => {
                let id = format!("m{obj_i:02}");
                obj_i += 1;
                id
            }
            SymbolKind::Tag => {
                let id = format!("t{tag_i:02x}");
                tag_i += 1;
                id
            }
            other => format!("x{:02}", other as u8),
        };
        symbols.push(Symbol {
            id,
            kind,
            qualified,
        });
    }

    for func in &functions {
        let qualified = format!("{pack}.{}.{}", func.class, func.name);
        if !symbols.iter().any(|s| s.qualified == qualified) {
            symbols.push(Symbol {
                id: format!(
                    "f{}_{}",
                    func.class.to_lowercase(),
                    func.name.to_lowercase()
                ),
                kind: SymbolKind::Function,
                qualified,
            });
        }
    }

    let mut chains = Vec::new();
    let mut blocks = Vec::new();
    let mut fills = Vec::new();
    let mut containers = Vec::new();
    let mut links = Vec::new();
    let mut origin = None;
    let mut dimension = None;
    let mut ticking_areas = Vec::new();
    let mut clock = None;
    let mut host_tick = None;
    let mut chain_places: Vec<WorldClause> = Vec::new();

    for item in &unit.items {
        match item {
            Item::Chain(chain) => {
                let annotations: Vec<String> =
                    chain.annotations.iter().map(|a| a.name.clone()).collect();
                let is_clock = annotations.iter().any(|n| n == "Repeat");
                symbols.push(Symbol {
                    id: chain.name.clone(),
                    kind: SymbolKind::Chain,
                    qualified: format!("{pack}.chain.{}", chain.name),
                });
                chains.push(ChainInfo {
                    name: chain.name.clone(),
                    annotations,
                    layout: None,
                    facing: None,
                    origin: None,
                    absolute: false,
                    bound: None,
                    is_clock,
                });
            }
            Item::World(world) => {
                for clause in &world.clauses {
                    match clause {
                        WorldClause::Origin { coords, .. } => origin = coords_3(coords),
                        WorldClause::Dimension(name) => dimension = Some(name.clone()),
                        WorldClause::Clock(name) => clock = Some(name.clone()),
                        WorldClause::HostTick(value) => host_tick = Some(value.clone()),
                        WorldClause::TickingArea {
                            name,
                            center,
                            radius,
                            preload,
                        } => ticking_areas.push(TickingAreaInfo {
                            name: name.clone(),
                            center: coords_3(center),
                            radius: radius.as_ref().and_then(expr_int),
                            preload: *preload,
                        }),
                        WorldClause::Link {
                            from_chain,
                            from_label,
                            to,
                        } => links.push(LinkInfo {
                            from_chain: from_chain.clone(),
                            from_label: from_label.clone(),
                            to: to.clone(),
                        }),
                        place @ WorldClause::ChainPlace { .. } => chain_places.push(place.clone()),
                        WorldClause::IncludePlace(_) => {}
                    }
                }
            }
            Item::Place(place) => {
                for stmt in &place.stmts {
                    match stmt {
                        PlaceStmt::BlockAt { block, at } | PlaceStmt::Set { block, at } => {
                            if let Some(at) = coords_3(at) {
                                blocks.push(PlacedBlock {
                                    block: block.dotted(),
                                    at,
                                });
                            }
                        }
                        PlaceStmt::Fill {
                            from,
                            to,
                            block,
                            replace,
                        } => {
                            if let (Some(from), Some(to)) = (coords_3(from), coords_3(to)) {
                                fills.push(FillOp {
                                    from,
                                    to,
                                    block: block.dotted(),
                                    replace: replace.as_ref().map(|p| p.dotted()),
                                });
                            }
                        }
                        PlaceStmt::Container {
                            kind,
                            name,
                            at,
                            facing,
                            slots,
                        } => {
                            if let Some(at) = coords_3(at) {
                                containers.push(ContainerInfo {
                                    kind: *kind,
                                    name: name.clone(),
                                    at,
                                    facing: facing.clone(),
                                    slots: slots
                                        .iter()
                                        .map(|s| SlotInfo {
                                            slot: s.slot,
                                            item: s.item.dotted(),
                                            count: s.count,
                                            data: s.data,
                                            title: s.title.clone(),
                                            pages: s.pages.clone(),
                                        })
                                        .collect(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    for place in chain_places {
        if let WorldClause::ChainPlace {
            name,
            at,
            layout,
            facing,
            absolute,
            bound,
        } = place
        {
            if let Some(chain) = chains.iter_mut().find(|c| c.name == name) {
                chain.layout = layout;
                chain.facing = facing;
                chain.origin = coords_3(&at);
                chain.absolute = absolute;
                chain.bound = bound.as_deref().and_then(coords_3);
            }
        }
    }
    if let Some(clock_name) = &clock {
        if let Some(chain) = chains.iter_mut().find(|c| c.name == *clock_name) {
            chain.is_clock = true;
        }
    }

    BinaryInfo {
        pack,
        score_revision,
        symbols,
        chains,
        blocks,
        fills,
        containers,
        links,
        origin,
        dimension,
        ticking_areas,
        clock,
        host_tick,
        functions,
    }
}

fn kind_of_field(field: &FieldDef) -> Option<SymbolKind> {
    match field.ty {
        TypeRef::Boolean if !field.is_static => Some(SymbolKind::Tag),
        TypeRef::Int | TypeRef::Boolean => Some(SymbolKind::Objective),
        _ => None,
    }
}

pub fn expr_int(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Int(n) => Some(*n),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
        } => expr_int(expr).map(|n| -n),
        _ => None,
    }
}

fn coords_3(coords: &[Expr]) -> Option<[i64; 3]> {
    match coords {
        [x, y, z] => Some([expr_int(x)?, expr_int(y)?, expr_int(z)?]),
        _ => None,
    }
}

impl BinaryInfo {
    pub fn symbol(&self, qualified: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.qualified == qualified)
    }

    pub fn field_id(&self, pack: &str, class: &str, field: &str) -> Option<&Symbol> {
        let q = format!("{pack}.{class}.{field}");
        self.symbols.iter().find(|s| s.qualified == q)
    }

    /// Human-readable dump used by tests and `minc inspect`.
    pub fn summary(&self) -> String {
        let mut out = format!("pack {}\n", self.pack);
        for sym in &self.symbols {
            let kind = match sym.kind {
                SymbolKind::Objective => "obj",
                SymbolKind::Tag => "tag",
                SymbolKind::FakePlayer => "fake",
                SymbolKind::Function => "fn",
                SymbolKind::Chain => "chain",
            };
            out.push_str(&format!("  {kind} {} {}\n", sym.id, sym.qualified));
        }
        for chain in &self.chains {
            out.push_str(&format!("  chain {}\n", chain.name));
        }
        out
    }
}
