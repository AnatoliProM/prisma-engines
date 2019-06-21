use super::filter::IntoFilter;
use connector::{filter::RecordFinder, write_query::*};
use prisma_models::prelude::*;
use std::sync::Arc;

pub fn convert_mutaction(m: crate::protobuf::prisma::DatabaseMutaction, project: ProjectRef) -> RootWriteQuery {
    use crate::protobuf::prisma::database_mutaction;
    match m.type_.unwrap() {
        database_mutaction::Type::Create(x) => convert_create_envelope(x, project),
        database_mutaction::Type::Update(x) => convert_update_envelope(x, project),
        database_mutaction::Type::Upsert(x) => convert_upsert(x, project),
        database_mutaction::Type::Delete(x) => convert_delete(x, project),
        database_mutaction::Type::Reset(x) => convert_reset(x, project),
        database_mutaction::Type::DeleteNodes(x) => convert_delete_nodes(x, project),
        database_mutaction::Type::UpdateNodes(x) => convert_update_nodes(x, project),
    }
}

pub fn convert_create_envelope(m: crate::protobuf::prisma::CreateNode, project: ProjectRef) -> RootWriteQuery {
    RootWriteQuery::CreateRecord(convert_create(m, project))
}

pub fn convert_create(m: crate::protobuf::prisma::CreateNode, project: ProjectRef) -> CreateRecord {
    let model = project.internal_data_model().find_model(&m.model_name).unwrap();
    CreateRecord {
        model,
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
        nested_writes: convert_nested_mutactions(m.nested, Arc::clone(&project)),
    }
}

pub fn convert_nested_mutactions(
    m: crate::protobuf::prisma::NestedMutactions,
    project: ProjectRef,
) -> NestedWriteQueries {
    NestedWriteQueries {
        creates: m
            .creates
            .into_iter()
            .map(|m| convert_nested_create(m, Arc::clone(&project)))
            .collect(),
        updates: m
            .updates
            .into_iter()
            .map(|m| convert_nested_update(m, Arc::clone(&project)))
            .collect(),
        upserts: m
            .upserts
            .into_iter()
            .map(|m| convert_nested_upsert(m, Arc::clone(&project)))
            .collect(),
        deletes: m
            .deletes
            .into_iter()
            .map(|m| convert_nested_delete(m, Arc::clone(&project)))
            .collect(),
        connects: m
            .connects
            .into_iter()
            .map(|m| convert_nested_connect(m, Arc::clone(&project)))
            .collect(),
        disconnects: m
            .disconnects
            .into_iter()
            .map(|m| convert_nested_disconnect(m, Arc::clone(&project)))
            .collect(),
        sets: m
            .sets
            .into_iter()
            .map(|m| convert_nested_set(m, Arc::clone(&project)))
            .collect(),
        update_manys: m
            .update_manys
            .into_iter()
            .map(|m| convert_nested_update_nodes(m, Arc::clone(&project)))
            .collect(),
        delete_manys: m
            .delete_manys
            .into_iter()
            .map(|m| convert_nested_delete_nodes(m, Arc::clone(&project)))
            .collect(),
    }
}

pub fn convert_nested_create(m: crate::protobuf::prisma::NestedCreateNode, project: ProjectRef) -> NestedCreateRecord {
    let relation_field = find_relation_field(Arc::clone(&project), m.model_name, m.field_name);

    NestedCreateRecord {
        relation_field: relation_field,
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
        top_is_create: m.top_is_create,
        nested_writes: convert_nested_mutactions(m.nested, Arc::clone(&project)),
    }
}

pub fn convert_update_envelope(m: crate::protobuf::prisma::UpdateNode, project: ProjectRef) -> RootWriteQuery {
    RootWriteQuery::UpdateRecord(convert_update(m, project))
}

pub fn convert_update(m: crate::protobuf::prisma::UpdateNode, project: ProjectRef) -> UpdateRecord {
    UpdateRecord {
        where_: convert_record_finder(m.where_, Arc::clone(&project)),
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
        nested_writes: convert_nested_mutactions(m.nested, Arc::clone(&project)),
    }
}

pub fn convert_nested_update(m: crate::protobuf::prisma::NestedUpdateNode, project: ProjectRef) -> NestedUpdateRecord {
    let relation_field = find_relation_field(Arc::clone(&project), m.model_name, m.field_name);
    NestedUpdateRecord {
        relation_field: relation_field,
        where_: m.where_.map(|w| convert_record_finder(w, Arc::clone(&project))),
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
        nested_writes: convert_nested_mutactions(m.nested, Arc::clone(&project)),
    }
}

pub fn convert_update_nodes(m: crate::protobuf::prisma::UpdateNodes, project: ProjectRef) -> RootWriteQuery {
    let model = project.internal_data_model().find_model(&m.model_name).unwrap();
    let update_nodes = UpdateManyRecords {
        model: Arc::clone(&model),
        filter: m.filter.into_filter(model),
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
    };

    RootWriteQuery::UpdateManyRecords(update_nodes)
}

pub fn convert_nested_update_nodes(
    m: crate::protobuf::prisma::NestedUpdateNodes,
    project: ProjectRef,
) -> NestedUpdateManyRecords {
    let relation_field = find_relation_field(Arc::clone(&project), m.model_name, m.field_name);
    NestedUpdateManyRecords {
        relation_field: Arc::clone(&relation_field),
        filter: m.filter.map(|f| f.into_filter(relation_field.related_model())),
        non_list_args: convert_prisma_args(m.non_list_args),
        list_args: convert_list_args(m.list_args),
    }
}

pub fn convert_upsert(m: crate::protobuf::prisma::UpsertNode, project: ProjectRef) -> RootWriteQuery {
    let upsert_node = UpsertRecord {
        where_: convert_record_finder(m.where_, Arc::clone(&project)),
        create: convert_create(m.create, Arc::clone(&project)),
        update: convert_update(m.update, project),
    };

    RootWriteQuery::UpsertRecord(upsert_node)
}

pub fn convert_nested_upsert(m: crate::protobuf::prisma::NestedUpsertNode, project: ProjectRef) -> NestedUpsertRecord {
    let relation_field = find_relation_field(Arc::clone(&project), m.model_name, m.field_name);
    NestedUpsertRecord {
        relation_field,
        where_: m.where_.map(|w| convert_record_finder(w, Arc::clone(&project))),
        create: convert_nested_create(m.create, Arc::clone(&project)),
        update: convert_nested_update(m.update, Arc::clone(&project)),
    }
}

pub fn convert_delete(m: crate::protobuf::prisma::DeleteNode, project: ProjectRef) -> RootWriteQuery {
    let delete_node = DeleteRecord {
        where_: convert_record_finder(m.where_, project),
    };

    RootWriteQuery::DeleteRecord(delete_node)
}

pub fn convert_nested_delete(m: crate::protobuf::prisma::NestedDeleteNode, project: ProjectRef) -> NestedDeleteRecord {
    NestedDeleteRecord {
        relation_field: find_relation_field(Arc::clone(&project), m.model_name, m.field_name),
        where_: m.where_.map(|w| convert_record_finder(w, project)),
    }
}

pub fn convert_delete_nodes(m: crate::protobuf::prisma::DeleteNodes, project: ProjectRef) -> RootWriteQuery {
    let model = project.internal_data_model().find_model(&m.model_name).unwrap();
    let delete_nodes = DeleteManyRecords {
        model: Arc::clone(&model),
        filter: m.filter.into_filter(model),
    };

    RootWriteQuery::DeleteManyRecords(delete_nodes)
}

pub fn convert_nested_delete_nodes(
    m: crate::protobuf::prisma::NestedDeleteNodes,
    project: ProjectRef,
) -> NestedDeleteManyRecords {
    let relation_field = find_relation_field(project, m.model_name, m.field_name);
    NestedDeleteManyRecords {
        relation_field: Arc::clone(&relation_field),
        filter: m.filter.map(|f| f.into_filter(relation_field.related_model())),
    }
}

pub fn convert_reset(_: crate::protobuf::prisma::ResetData, project: ProjectRef) -> RootWriteQuery {
    let mutaction = ResetData {
        internal_data_model: project.internal_data_model_ref(),
    };

    RootWriteQuery::ResetData(mutaction)
}

pub fn convert_nested_connect(m: crate::protobuf::prisma::NestedConnect, project: ProjectRef) -> NestedConnect {
    let relation_field = project
        .internal_data_model()
        .find_model(&m.model_name)
        .unwrap()
        .fields()
        .find_from_relation_fields(&m.field_name)
        .unwrap();

    NestedConnect {
        relation_field,
        where_: convert_record_finder(m.where_, project),
        top_is_create: m.top_is_create,
    }
}

pub fn convert_nested_disconnect(
    m: crate::protobuf::prisma::NestedDisconnect,
    project: ProjectRef,
) -> NestedDisconnect {
    let relation_field = project
        .internal_data_model()
        .find_model(&m.model_name)
        .unwrap()
        .fields()
        .find_from_relation_fields(&m.field_name)
        .unwrap();

    NestedDisconnect {
        relation_field,
        where_: m.where_.map(|w| convert_record_finder(w, project)),
    }
}

pub fn convert_nested_set(m: crate::protobuf::prisma::NestedSet, project: ProjectRef) -> NestedSet {
    let relation_field = project
        .internal_data_model()
        .find_model(&m.model_name)
        .unwrap()
        .fields()
        .find_from_relation_fields(&m.field_name)
        .unwrap();

    NestedSet {
        relation_field,
        wheres: m
            .wheres
            .into_iter()
            .map(|w| convert_record_finder(w, Arc::clone(&project)))
            .collect(),
    }
}

pub fn convert_record_finder(selector: crate::protobuf::prisma::NodeSelector, project: ProjectRef) -> RecordFinder {
    let model = project.internal_data_model().find_model(&selector.model_name).unwrap();
    let field = model.fields().find_from_scalar(&selector.field_name).unwrap();
    let value: PrismaValue = selector.value.into();
    RecordFinder { field, value }
}

pub fn convert_prisma_args(proto: crate::protobuf::prisma::PrismaArgs) -> PrismaArgs {
    let mut result = PrismaArgs::default();
    for arg in proto.args {
        result.insert(arg.key, arg.value);
    }
    result
}

pub fn convert_list_args(proto: crate::protobuf::prisma::PrismaArgs) -> Vec<(String, PrismaListValue)> {
    let mut result = vec![];
    for arg in proto.args {
        let value: PrismaListValue = arg.value.into();
        let tuple = (arg.key, value);
        result.push(tuple)
    }
    result
}

pub fn find_relation_field(project: ProjectRef, model: String, field: String) -> Arc<RelationField> {
    project
        .internal_data_model()
        .find_model(&model)
        .unwrap()
        .fields()
        .find_from_relation_fields(&field)
        .unwrap()
}

pub fn convert_mutaction_result(result: WriteQueryResult) -> crate::protobuf::prisma::DatabaseMutactionResult {
    use crate::protobuf::prisma::database_mutaction_result;

    match result.typ {
        WriteQueryResultType::Create => {
            let result = crate::protobuf::prisma::IdResult { id: result.id().into() };
            let typ = database_mutaction_result::Type::Create(result);

            crate::protobuf::prisma::DatabaseMutactionResult { type_: Some(typ) }
        }
        WriteQueryResultType::Update => {
            let result = crate::protobuf::prisma::IdResult { id: result.id().into() };
            let typ = database_mutaction_result::Type::Update(result);

            crate::protobuf::prisma::DatabaseMutactionResult { type_: Some(typ) }
        }
        WriteQueryResultType::Delete => {
            let result = crate::protobuf::prisma::NodeResult::from(result.record().clone());
            let typ = database_mutaction_result::Type::Delete(result);

            crate::protobuf::prisma::DatabaseMutactionResult { type_: Some(typ) }
        }
        WriteQueryResultType::Many => {
            let result = crate::protobuf::prisma::ManyNodesResult {
                count: result.count() as u32,
            };
            let typ = database_mutaction_result::Type::Many(result);
            crate::protobuf::prisma::DatabaseMutactionResult { type_: Some(typ) }
        }
        WriteQueryResultType::Unit => {
            let result = crate::protobuf::prisma::Unit {};
            let typ = database_mutaction_result::Type::Unit(result);
            crate::protobuf::prisma::DatabaseMutactionResult { type_: Some(typ) }
        }

        // x => panic!("can't handle result type {:?}", x),
    }
}
