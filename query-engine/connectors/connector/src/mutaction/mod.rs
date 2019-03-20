mod builder;
mod create_node;
mod node_address;
mod path;
mod relay_id;
mod result;

pub use builder::*;
pub use create_node::*;
pub use node_address::*;
pub use path::*;
pub use relay_id::*;
pub use result::*;

pub trait NestedMutaction {
    fn nested_mutactions(&self) -> &[&DatabaseMutaction];
}

#[derive(Debug, Clone)]
pub enum DatabaseMutaction {
    TopLevel(TopLevelDatabaseMutaction),
    Nested(NestedDatabaseMutaction),
}

#[derive(Debug, Clone)]
pub enum TopLevelDatabaseMutaction {
    CreateNode(CreateNode),
}

#[derive(Debug, Clone)]
pub enum NestedDatabaseMutaction {
    CreateNode(NestedCreateNode),
}

#[derive(Debug, Default, Clone)]
pub struct NestedMutactions {
    pub creates: Vec<NestedCreateNode>,
    /*
    pub updates: Vec<NestedUpdateNode>,
    pub upserts: Vec<NestedUpsertNode>,
    pub deletes: Vec<NestedDeleteNode>,
    pub connects: Vec<NestedConnect>,
    pub sets: Vec<NestedSet>,
    pub disconnects: Vec<NestedDisconnect>,
    pub update_manys: Vec<NestedUpdateNodes>,
    pub delete_manys: Vec<NestedDeleteNodes>,
    */
}

// UPDATE

/*
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TopLevelUpdateNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
    pub nested_mutactions: NestedMutactions,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedUpdateNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,
    pub non_list_args: PrismaArgs,
    pub list_args: Vec<(String, PrismaListValue)>,
    pub nested_mutactions: NestedMutactions,

    pub relation_field: Arc<RelationField>,
}

// UPSERT

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TopLevelUpsertNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,
    pub create: TopLevelCreateNode,
    pub update: TopLevelUpdateNode,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedUpsertNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,
    pub create: NestedCreateNode,
    pub update: NestedUpdateNode,

    pub relation_field: Arc<RelationField>,
}

// DELETE

#[derive(Debug, Default, Clone, PartialEq)]
pub struct TopLevelDeleteNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,
    pub previous_values: Node,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedDeleteNode {
    pub project: ProjectRef,
    pub where_: NodeSelector,

    pub relation_field: Arc<RelationField>,
}

// CONNECT

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedConnect {
    pub project: ProjectRef,
    pub where_: NodeSelector,

    pub relation_field: Arc<RelationField>,
    pub top_is_create: bool,
}

// SET

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedSet {
    pub project: ProjectRef,
    pub wheres: Vec<NodeSelector>,

    pub relation_field: Arc<RelationField>,
}

// DISCONNECT

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NestedDisconnect {
    pub project: ProjectRef,
    pub where_: Option<NodeSelector>,

    pub relation_field: Arc<RelationField>,
}*/
