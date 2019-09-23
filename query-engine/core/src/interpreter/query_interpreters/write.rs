use crate::{interpreter::InterpretationResult, query_ast::*, QueryResult};
use connector::{Filter, TransactionLike, WriteArgs};

pub fn execute(tx: &mut dyn TransactionLike, write_query: WriteQuery) -> InterpretationResult<QueryResult> {
    match write_query {
        WriteQuery::CreateRecord(q) => create_one(tx, q),
        WriteQuery::UpdateRecord(q) => update_one(tx, q),
        WriteQuery::DeleteRecord(q) => delete_one(tx, q),
        WriteQuery::UpdateManyRecords(q) => update_many(tx, q),
        WriteQuery::DeleteManyRecords(q) => delete_many(tx, q),
        WriteQuery::ConnectRecords(q) => connect(tx, q),
        WriteQuery::DisconnectRecords(q) => disconnect(tx, q),
        WriteQuery::SetRecords(q) => set(tx, q),
        WriteQuery::ResetData(q) => reset(tx, q),
    }
}

fn create_one(tx: &mut dyn TransactionLike, q: CreateRecord) -> InterpretationResult<QueryResult> {
    let res = tx.create_record(q.model, WriteArgs::new(q.non_list_args, q.list_args))?;
    Ok(QueryResult::Id(res))
}

fn update_one(tx: &mut dyn TransactionLike, q: UpdateRecord) -> InterpretationResult<QueryResult> {
    let mut res = tx.update_records(
        q.model,
        Filter::from(q.where_),
        WriteArgs::new(q.non_list_args, q.list_args),
    )?;

    Ok(QueryResult::Id(res.pop().unwrap()))
}

fn delete_one(tx: &mut dyn TransactionLike, q: DeleteRecord) -> InterpretationResult<QueryResult> {
    let res = tx.delete_records(q.model, Filter::from(q.where_))?;
    Ok(QueryResult::Count(res))
}

fn update_many(tx: &mut dyn TransactionLike, q: UpdateManyRecords) -> InterpretationResult<QueryResult> {
    let res = tx.update_records(q.model, q.filter, WriteArgs::new(q.non_list_args, q.list_args))?;
    Ok(QueryResult::Count(res.len()))
}

fn delete_many(tx: &mut dyn TransactionLike, q: DeleteManyRecords) -> InterpretationResult<QueryResult> {
    let res = tx.delete_records(q.model, q.filter)?;
    Ok(QueryResult::Count(res))
}

fn connect(tx: &mut dyn TransactionLike, q: ConnectRecords) -> InterpretationResult<QueryResult> {
    tx.connect(
        q.relation_field,
        &q.parent.expect("Expected parent record ID to be set for connect"),
        &q.child.expect("Expected child record ID to be set for connect"),
    )?;

    Ok(QueryResult::Unit)
}

fn disconnect(_tx: &mut dyn TransactionLike, _q: DisconnectRecords) -> InterpretationResult<QueryResult> {
    unimplemented!()
}

fn set(_tx: &mut dyn TransactionLike, _q: SetRecords) -> InterpretationResult<QueryResult> {
    unimplemented!()
}

fn reset(_tx: &mut dyn TransactionLike, _q: ResetData) -> InterpretationResult<QueryResult> {
    unimplemented!()
}
