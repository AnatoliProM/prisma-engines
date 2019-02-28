use super::FieldManifestation;
use crate::prelude::*;
use once_cell::unsync::OnceCell;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationFieldTemplate {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_unique: bool,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub is_auto_generated: bool,
    pub manifestation: Option<FieldManifestation>,
    pub relation_name: String,
    pub relation_side: RelationSide,
}

#[derive(DebugStub)]
pub struct RelationField {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_unique: bool,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub is_auto_generated: bool,
    pub relation_name: String,
    pub relation_side: RelationSide,
    #[debug_stub = "#ModelWeakRef#"]
    pub model: ModelWeakRef,
    pub relation: OnceCell<RelationWeakRef>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum RelationSide {
    A,
    B,
}

impl RelationSide {
    pub fn opposite(self) -> RelationSide {
        match self {
            RelationSide::A => RelationSide::B,
            RelationSide::B => RelationSide::A,
        }
    }
}

impl RelationField {
    fn relation(&self) -> Option<RelationRef> {
        self.relation
            .get_or_init(|| {
                self.with_model(|model| {
                    model.with_schema(|schema| schema.find_relation(&self.relation_name).unwrap())
                })
            })
            .upgrade()
    }

    pub fn with_relation<F, T>(&self, f: F) -> T
    where
        F: FnOnce(RelationRef) -> T,
    {
        match self.relation() {
            Some(relation) => f(relation),
            None => panic!(
                "Relation does not exist anymore. Parent relation is deleted without deleting the child fields."
            )
        }
    }

    fn with_model<F, T>(&self, f: F) -> T
    where
        F: FnOnce(ModelRef) -> T,
    {
        match self.model.upgrade() {
            Some(model) => f(model),
            None => panic!(
                "Model does not exist anymore. Parent model is deleted without deleting the child fields."
            )
        }
    }

    pub fn db_name(&self) -> String {
        use RelationLinkManifestation::*;

        self.with_model(|model| {
            self.with_relation(|relation| match relation.manifestation {
                Some(Inline(ref m)) => {
                    let is_self_rel = relation.is_self_relation();

                    if is_self_rel
                        && (self.relation_side == RelationSide::B || self.related_field().is_hidden)
                    {
                        m.referencing_column.clone()
                    } else if is_self_rel && self.relation_side == RelationSide::A {
                        self.name.clone()
                    } else if m.in_table_of_model_name == model.name {
                        m.referencing_column.clone()
                    } else {
                        self.name.clone()
                    }
                }
                _ => self.name.clone(),
            })
        })
    }

    pub fn related_model(&self) -> ModelRef {
        self.with_relation(|relation| match self.relation_side {
            RelationSide::A => relation.model_b(),
            RelationSide::B => relation.model_a(),
        })
    }

    pub fn related_field(&self) -> Arc<RelationField> {
        self.with_relation(|relation| match self.relation_side {
            RelationSide::A => relation.field_b(),
            RelationSide::B => relation.field_a(),
        })
    }

    pub fn is_relation_with_name_and_side(&self, relation_name: &str, side: RelationSide) -> bool {
        self.with_relation(|relation| relation.name == relation_name && self.relation_side == side)
    }
}
